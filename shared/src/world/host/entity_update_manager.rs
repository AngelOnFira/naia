use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    net::SocketAddr,
};
use std::sync::RwLockReadGuard;
use std::time::Duration;

use super::user_diff_handler::UserDiffHandler;
use crate::{ComponentKind, EntityAndGlobalEntityConverter, GlobalEntity, GlobalWorldManagerType, Instant, WorldRefType, DiffMask, PacketIndex};
use crate::world::host::checked_map::{CheckedMap, CheckedSet};
use crate::world::sync::EntityChannelReceiver;

const DROP_UPDATE_RTT_FACTOR: f32 = 1.5;

pub struct EntityUpdateManager {
    address: Option<SocketAddr>,
    diff_handler: UserDiffHandler,
    sent_updates: HashMap<PacketIndex, (Instant, HashMap<(GlobalEntity, ComponentKind), DiffMask>)>,
    last_update_packet_index: PacketIndex,
}

impl EntityUpdateManager {
    pub fn new(
        address: &Option<SocketAddr>,
        global_world_manager: &dyn GlobalWorldManagerType,
    ) -> Self {
        Self {
            address: *address,
            diff_handler: UserDiffHandler::new(global_world_manager),
            sent_updates: HashMap::new(),
            last_update_packet_index: 0,
        }
    }

    // Main

    pub fn diff_handler_has_component(&self, entity: &GlobalEntity, component_kind: &ComponentKind) -> bool {
        self.diff_handler.has_component(entity, component_kind)
    }

    pub fn or_diff_mask(&mut self, entity: &GlobalEntity, component_kind: &ComponentKind, new_diff_mask: &DiffMask) {
        self.diff_handler.or_diff_mask(entity, component_kind, new_diff_mask);
    }

    pub fn get_diff_mask(&self, entity: &GlobalEntity, component_kind: &ComponentKind) -> RwLockReadGuard<DiffMask> {
        self.diff_handler.diff_mask(entity, component_kind)
    }

    pub fn clear_diff_mask(&mut self, entity: &GlobalEntity, component_kind: &ComponentKind) {
        self.diff_handler.clear_diff_mask(entity, component_kind);
    }

    pub fn register_component(
        &mut self,
        entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.diff_handler.register_component(&self.address, entity, component_kind);
    }

    pub fn deregister_component(
        &mut self,
        entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.diff_handler.deregister_component(entity, component_kind);
    }

    // Collect

    pub fn collect_next_updates<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        &self,
        world: &W,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        host_world: &CheckedMap<GlobalEntity, CheckedSet<ComponentKind>>,
        remote_world: &HashMap<GlobalEntity, EntityChannelReceiver>,
    ) -> HashMap<GlobalEntity, HashSet<ComponentKind>> {
        let mut output = HashMap::new();

        for (global_entity, host_components) in host_world.iter() {
            
            let Some(entity_channel) = remote_world.get(global_entity) else {
                continue;
            };

            let Ok(world_entity) = converter.global_entity_to_entity(global_entity) else {
                panic!("World Channel: cannot convert global entity ({:?}) to world entity", global_entity);
            };
            if !world.has_entity(&world_entity) {
                continue;
            }
            for component_kind in host_components.iter() {
                if !entity_channel.has_component_kind(component_kind) {
                    continue;
                }
                if self
                    .diff_handler
                    .diff_mask_is_clear(global_entity, component_kind)
                {
                    continue;
                }
                let entity_is_replicating =
                    global_world_manager.entity_is_replicating(global_entity);
                let world_has_component =
                    world.has_component_of_kind(&world_entity, component_kind);
                if entity_is_replicating && world_has_component {
                    if !output.contains_key(global_entity) {
                        output.insert(*global_entity, HashSet::new());
                    }
                    let send_component_set = output.get_mut(global_entity).unwrap();
                    send_component_set.insert(*component_kind);
                }
            }
        }
        output
    }

    pub fn handle_dropped_update_packets(&mut self, now: &Instant, rtt_millis: &f32) {
        let drop_duration = Duration::from_millis((DROP_UPDATE_RTT_FACTOR * rtt_millis) as u64);

        {
            let mut dropped_packets = Vec::new();
            for (packet_index, (time_sent, _)) in &self.sent_updates {
                let elapsed_since_send = time_sent.elapsed(now);
                if elapsed_since_send > drop_duration {
                    dropped_packets.push(*packet_index);
                }
            }

            for packet_index in dropped_packets {
                self.dropped_update_cleanup(packet_index);
            }
        }
    }

    fn dropped_update_cleanup(&mut self, dropped_packet_index: PacketIndex) {
        if let Some((_, diff_mask_map)) = self.sent_updates.remove(&dropped_packet_index) {
            for (component_index, diff_mask) in &diff_mask_map {
                let (entity, component) = component_index;
                if !self
                    .diff_handler_has_component(entity, component)
                {
                    continue;
                }
                let mut new_diff_mask = diff_mask.clone();

                // walk from dropped packet up to most recently sent packet
                if dropped_packet_index != self.last_update_packet_index {
                    let mut packet_index = dropped_packet_index.wrapping_add(1);
                    while packet_index != self.last_update_packet_index {
                        if let Some((_, diff_mask_map)) = self.sent_updates.get(&packet_index) {
                            if let Some(next_diff_mask) = diff_mask_map.get(component_index) {
                                new_diff_mask.nand(next_diff_mask);
                            }
                        }

                        packet_index = packet_index.wrapping_add(1);
                    }
                }

                self.or_diff_mask(entity, component, &new_diff_mask);
            }
        }
    }

    pub fn notify_packet_delivered(
        &mut self,
        packet_index: PacketIndex,
    ) {
        self.sent_updates.remove(&packet_index);
    }

    pub fn record_update(
        &mut self,
        now: &Instant,
        packet_index: &PacketIndex,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
        diff_mask: DiffMask
    ) {
        self.last_update_packet_index = *packet_index;

        // place diff mask in a special transmission record - like map
        if !self.sent_updates.contains_key(packet_index) {
            self
                .sent_updates
                .insert(*packet_index, (now.clone(), HashMap::new()));
        }
        let (_, sent_updates_map) = self.sent_updates.get_mut(packet_index).unwrap();
        sent_updates_map.insert((*global_entity, *component_kind), diff_mask.clone());

        // having copied the diff mask for this update, clear the component
        self.clear_diff_mask(global_entity, component_kind);
    }
}
