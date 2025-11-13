use std::{
    clone::Clone,
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
};

use crate::{
    messages::channels::senders::indexed_message_writer::IndexedMessageWriter,
    sequence_list::SequenceList,
    world::{
        entity::entity_converters::GlobalWorldManagerType, local_world_manager::LocalWorldManager,
        host::error::WorldChannelError,
    },
    BitWrite, BitWriter, ComponentKind, ComponentKinds, ConstBitLength, EntityAction,
    EntityActionType, EntityAndLocalEntityConverter, EntityConverterMut, HostWorldEvents,
    HostWorldManager, Instant, MessageIndex, PacketIndex, Serde, UnsignedVariableInteger,
    WorldRefType,
};

use super::entity_action_event::EntityActionEvent;

pub type ActionId = MessageIndex;

pub struct HostWorldWriter;

impl HostWorldWriter {
    fn write_action_id(
        writer: &mut dyn BitWrite,
        last_id_opt: &mut Option<ActionId>,
        current_id: &ActionId,
    ) {
        IndexedMessageWriter::write_message_index(writer, last_id_opt, current_id);
        *last_id_opt = Some(*current_id);
    }

    pub fn write_into_packet<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        now: &Instant,
        writer: &mut BitWriter,
        packet_index: &PacketIndex,
        world: &W,
        global_world_manager: &dyn GlobalWorldManagerType<E>,
        local_world_manager: &mut LocalWorldManager<E>,
        has_written: &mut bool,
        host_manager: &mut HostWorldManager<E>,
        world_events: &mut HostWorldEvents<E>,
    ) {
        // write entity updates
        Self::write_updates(
            component_kinds,
            now,
            writer,
            &packet_index,
            world,
            global_world_manager,
            local_world_manager,
            has_written,
            host_manager,
            &mut world_events.next_send_updates,
        );

        // write entity actions
        Self::write_actions(
            component_kinds,
            now,
            writer,
            &packet_index,
            world,
            global_world_manager,
            local_world_manager,
            has_written,
            host_manager,
            &mut world_events.next_send_actions,
        );
    }

    fn write_actions<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        now: &Instant,
        writer: &mut BitWriter,
        packet_index: &PacketIndex,
        world: &W,
        global_world_manager: &dyn GlobalWorldManagerType<E>,
        local_world_manager: &mut LocalWorldManager<E>,
        has_written: &mut bool,
        host_manager: &mut HostWorldManager<E>,
        next_send_actions: &mut VecDeque<(ActionId, EntityActionEvent<E>)>,
    ) {
        let mut last_counted_id: Option<MessageIndex> = None;
        let mut last_written_id: Option<MessageIndex> = None;

        loop {
            if next_send_actions.is_empty() {
                break;
            }

            // check that we can write the next message
            let mut counter = writer.counter();
            // write ActionContinue bit
            true.ser(&mut counter);
            // write data
            if let Err(e) = Self::write_action(
                component_kinds,
                world,
                global_world_manager,
                local_world_manager,
                packet_index,
                &mut counter,
                &mut last_counted_id,
                false,
                host_manager,
                next_send_actions,
            ) {
                // Component not found during action write, skip this action
                log::error!("Skipping action due to error: {}", e);
                next_send_actions.pop_front();
                continue;
            }
            if counter.overflowed() {
                // if nothing useful has been written in this packet yet,
                // log error about size of component being too big
                if !*has_written {
                    let error = Self::create_overflow_error(
                        component_kinds,
                        counter.bits_needed(),
                        writer.bits_free(),
                        next_send_actions,
                    );
                    log::error!("Packet write overflow (skipping action): {}", error);
                }
                break;
            }

            *has_written = true;

            // optimization
            if !host_manager
                .sent_action_packets
                .contains_scan_from_back(packet_index)
            {
                host_manager
                    .sent_action_packets
                    .insert_scan_from_back(*packet_index, (now.clone(), Vec::new()));
            }

            // write ActionContinue bit
            true.ser(writer);
            // write data
            if let Err(e) = Self::write_action(
                component_kinds,
                world,
                global_world_manager,
                local_world_manager,
                packet_index,
                writer,
                &mut last_written_id,
                true,
                host_manager,
                next_send_actions,
            ) {
                // Component not found during action write, skip this action
                log::error!("Skipping action due to error: {}", e);
                next_send_actions.pop_front();
                continue;
            }

            // pop action we've written
            next_send_actions.pop_front();
        }

        // Finish actions by writing false ActionContinue bit
        writer.release_bits(1);
        false.ser(writer);
    }

    #[allow(clippy::too_many_arguments)]
    fn write_action<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        world: &W,
        global_world_manager: &dyn GlobalWorldManagerType<E>,
        local_world_manager: &mut LocalWorldManager<E>,
        packet_index: &PacketIndex,
        writer: &mut dyn BitWrite,
        last_written_id: &mut Option<ActionId>,
        is_writing: bool,
        host_manager: &mut HostWorldManager<E>,
        next_send_actions: &mut VecDeque<(ActionId, EntityActionEvent<E>)>,
    ) -> Result<(), WorldChannelError> {
        // SAFETY: This function is only called when next_send_actions is not empty,
        // as checked in write_actions loop before calling this function
        let (action_id, action) = next_send_actions.front().unwrap();

        // write message id
        Self::write_action_id(writer, last_written_id, action_id);

        match action {
            EntityActionEvent::SpawnEntity(world_entity, component_kind_list) => {
                EntityActionType::SpawnEntity.ser(writer);

                // write net entity
                // SAFETY: If we are writing a SpawnEntity action, the entity must already be
                // registered in local_world_manager by the time it reaches the action queue
                let host_entity = local_world_manager
                    .entity_to_host_entity(world_entity)
                    .unwrap();
                host_entity.ser(writer);

                // write number of components
                let components_num =
                    UnsignedVariableInteger::<3>::new(component_kind_list.len() as i128);
                components_num.ser(writer);

                for component_kind in component_kind_list {
                    let mut converter =
                        EntityConverterMut::new(global_world_manager, local_world_manager);

                    // write component payload
                    let component = world
                        .component_of_kind(world_entity, component_kind)
                        .ok_or_else(|| WorldChannelError::ComponentNotFoundDuringWrite {
                            entity_id: "<entity>".to_string(),
                            component_kind: component_kinds.kind_to_name(component_kind),
                        })?;
                    component.write(component_kinds, writer, &mut converter);
                }

                // if we are writing to this packet, add it to record
                if is_writing {
                    Self::record_action_written(
                        &mut host_manager.sent_action_packets,
                        packet_index,
                        action_id,
                        EntityAction::SpawnEntity(*world_entity, component_kind_list.clone()),
                    );
                }
            }
            EntityActionEvent::DespawnEntity(world_entity) => {
                EntityActionType::DespawnEntity.ser(writer);

                // write net entity
                // SAFETY: If we are writing a DespawnEntity action, the entity must still be
                // registered in local_world_manager (it will be removed after write completes)
                let host_entity = local_world_manager
                    .entity_to_host_entity(world_entity)
                    .unwrap();
                host_entity.ser(writer);

                // if we are writing to this packet, add it to record
                if is_writing {
                    Self::record_action_written(
                        &mut host_manager.sent_action_packets,
                        packet_index,
                        action_id,
                        EntityAction::DespawnEntity(*world_entity),
                    );
                }
            }
            EntityActionEvent::InsertComponent(world_entity, component) => {
                if !host_manager
                    .world_channel
                    .entity_channel_is_open(world_entity)
                    || !world.has_component_of_kind(world_entity, component)
                {
                    EntityActionType::Noop.ser(writer);

                    // if we are actually writing this packet
                    if is_writing {
                        // add it to action record
                        Self::record_action_written(
                            &mut host_manager.sent_action_packets,
                            packet_index,
                            action_id,
                            EntityAction::Noop,
                        );
                    }
                } else {
                    EntityActionType::InsertComponent.ser(writer);

                    // write net entity
                    // SAFETY: If we are writing an InsertComponent action and entity channel is open,
                    // the entity must be registered in local_world_manager
                    let host_entity = local_world_manager
                        .entity_to_host_entity(world_entity)
                        .unwrap();
                    host_entity.ser(writer);

                    let mut converter =
                        EntityConverterMut::new(global_world_manager, local_world_manager);

                    // write component payload
                    let comp = world
                        .component_of_kind(world_entity, component)
                        .ok_or_else(|| WorldChannelError::ComponentNotFoundDuringWrite {
                            entity_id: "<entity>".to_string(),
                            component_kind: component_kinds.kind_to_name(component),
                        })?;
                    comp.write(component_kinds, writer, &mut converter);

                    // if we are actually writing this packet
                    if is_writing {
                        // add it to action record
                        Self::record_action_written(
                            &mut host_manager.sent_action_packets,
                            packet_index,
                            action_id,
                            EntityAction::InsertComponent(*world_entity, *component),
                        );
                    }
                }
            }
            EntityActionEvent::RemoveComponent(world_entity, component_kind) => {
                if !host_manager
                    .world_channel
                    .entity_channel_is_open(world_entity)
                {
                    EntityActionType::Noop.ser(writer);

                    // if we are actually writing this packet
                    if is_writing {
                        // add it to action record
                        Self::record_action_written(
                            &mut host_manager.sent_action_packets,
                            packet_index,
                            action_id,
                            EntityAction::Noop,
                        );
                    }
                } else {
                    EntityActionType::RemoveComponent.ser(writer);

                    // write net entity
                    // SAFETY: If we are writing a RemoveComponent action and entity channel is open,
                    // the entity must be registered in local_world_manager
                    let host_entity = local_world_manager
                        .entity_to_host_entity(world_entity)
                        .unwrap();
                    host_entity.ser(writer);

                    // write component kind
                    component_kind.ser(component_kinds, writer);

                    // if we are writing to this packet, add it to record
                    if is_writing {
                        Self::record_action_written(
                            &mut host_manager.sent_action_packets,
                            packet_index,
                            action_id,
                            EntityAction::RemoveComponent(*world_entity, *component_kind),
                        );
                    }
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn record_action_written<E: Copy + Eq + Hash + Send + Sync>(
        sent_actions: &mut SequenceList<(Instant, Vec<(ActionId, EntityAction<E>)>)>,
        packet_index: &PacketIndex,
        action_id: &ActionId,
        action_record: EntityAction<E>,
    ) {
        // SAFETY: packet_index was inserted into sent_action_packets in write_actions
        // before this function is called (see optimization check at line ~130-137)
        let (_, sent_actions_list) = sent_actions.get_mut_scan_from_back(packet_index).unwrap();
        sent_actions_list.push((*action_id, action_record));
    }

    fn create_overflow_error<E: Copy + Eq + Hash + Send + Sync>(
        component_kinds: &ComponentKinds,
        bits_needed: u32,
        bits_free: u32,
        next_send_actions: &VecDeque<(ActionId, EntityActionEvent<E>)>,
    ) -> WorldChannelError {
        // SAFETY: This function is only called when next_send_actions is not empty,
        // as checked by the caller in write_actions loop
        let (_action_id, action) = next_send_actions.front().unwrap();

        match action {
            EntityActionEvent::SpawnEntity(_entity, component_kind_list) => {
                let mut component_names = String::new();
                let mut added = false;

                for component_kind in component_kind_list {
                    if added {
                        component_names.push(',');
                    } else {
                        added = true;
                    }
                    let name = component_kinds.kind_to_name(component_kind);
                    component_names.push_str(&name);
                }
                WorldChannelError::EntitySpawnPacketOverflow {
                    entity_id: "<entity>".to_string(),
                    component_names,
                    bits_needed,
                    bits_free,
                }
            }
            EntityActionEvent::InsertComponent(_entity, component_kind) => {
                let component_name = component_kinds.kind_to_name(component_kind);
                WorldChannelError::ComponentInsertPacketOverflow {
                    entity_id: "<entity>".to_string(),
                    component_kind: component_name,
                    bits_needed,
                    bits_free,
                }
            }
            _ => WorldChannelError::ActionPacketOverflow {
                bits_needed,
                bits_free,
            },
        }
    }

    fn write_updates<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        now: &Instant,
        writer: &mut BitWriter,
        packet_index: &PacketIndex,
        world: &W,
        global_world_manager: &dyn GlobalWorldManagerType<E>,
        local_world_manager: &mut LocalWorldManager<E>,
        has_written: &mut bool,
        host_manager: &mut HostWorldManager<E>,
        next_send_updates: &mut HashMap<E, HashSet<ComponentKind>>,
    ) {
        let all_update_entities: Vec<E> = next_send_updates.keys().copied().collect();

        for entity in all_update_entities {
            // get LocalEntity
            // SAFETY: If an entity is in next_send_updates, it must be tracked by local_world_manager
            // as entities are only added to updates after they have been spawned and registered
            let host_entity = local_world_manager.entity_to_host_entity(&entity).unwrap();

            // check that we can at least write a LocalEntity and a ComponentContinue bit
            let mut counter = writer.counter();
            // reserve ComponentContinue bit
            counter.write_bit(true);
            // write UpdateContinue bit
            counter.write_bit(true);
            // write LocalEntity
            host_entity.ser(&mut counter);
            if counter.overflowed() {
                break;
            }

            // reserve ComponentContinue bit
            writer.reserve_bits(1);
            // write UpdateContinue bit
            true.ser(writer);
            // write HostEntity
            host_entity.ser(writer);
            // write Components
            if let Err(e) = Self::write_update(
                component_kinds,
                now,
                world,
                global_world_manager,
                local_world_manager,
                packet_index,
                writer,
                &entity,
                has_written,
                host_manager,
                next_send_updates,
            ) {
                // Component not found during update write, skip remaining components for this entity
                log::error!("Skipping entity update due to error: {}", e);
                // Remove this entity from next_send_updates to avoid retrying
                next_send_updates.remove(&entity);
            }

            // write ComponentContinue finish bit, release
            writer.release_bits(1);
            false.ser(writer);
        }

        // write EntityContinue finish bit, release
        writer.release_bits(1);
        false.ser(writer);
    }

    /// For a given entity, write component value updates into a packet
    /// Only component values that changed in the internal (naia's) host world will be written
    fn write_update<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        component_kinds: &ComponentKinds,
        now: &Instant,
        world: &W,
        global_world_manager: &dyn GlobalWorldManagerType<E>,
        local_world_manager: &mut LocalWorldManager<E>,
        packet_index: &PacketIndex,
        writer: &mut BitWriter,
        entity: &E,
        has_written: &mut bool,
        host_manager: &mut HostWorldManager<E>,
        next_send_updates: &mut HashMap<E, HashSet<ComponentKind>>,
    ) -> Result<(), WorldChannelError> {
        let mut written_component_kinds = Vec::new();
        // SAFETY: write_update is only called from write_updates with entities from next_send_updates.keys()
        let component_kind_set = next_send_updates.get(entity).unwrap();
        for component_kind in component_kind_set {
            // get diff mask
            let diff_mask = host_manager
                .world_channel
                .diff_handler
                .diff_mask(entity, component_kind)
                .clone();

            let mut converter = EntityConverterMut::new(global_world_manager, local_world_manager);

            // check that we can write the next component update
            let mut counter = writer.counter();
            // write ComponentContinue bit
            true.ser(&mut counter);
            // write component kind
            counter.count_bits(<ComponentKind as ConstBitLength>::const_bit_length());
            // write data
            let component = world
                .component_of_kind(entity, component_kind)
                .ok_or_else(|| WorldChannelError::ComponentNotFoundDuringWrite {
                    entity_id: "<entity>".to_string(),
                    component_kind: component_kinds.kind_to_name(component_kind),
                })?;
            component.write_update(&diff_mask, &mut counter, &mut converter);
            if counter.overflowed() {
                // if nothing useful has been written in this packet yet,
                // log error about size of component being too big
                if !*has_written {
                    let component_name = component_kinds.kind_to_name(component_kind);
                    let error = Self::create_update_overflow_error(
                        component_name,
                        counter.bits_needed(),
                        writer.bits_free(),
                    );
                    log::error!("Packet write overflow (skipping component update): {}", error);
                }

                break;
            }

            *has_written = true;

            // write ComponentContinue bit
            true.ser(writer);
            // write component kind
            component_kind.ser(component_kinds, writer);
            // write data
            let component = world
                .component_of_kind(entity, component_kind)
                .ok_or_else(|| WorldChannelError::ComponentNotFoundDuringWrite {
                    entity_id: "<entity>".to_string(),
                    component_kind: component_kinds.kind_to_name(component_kind),
                })?;
            component.write_update(&diff_mask, writer, &mut converter);

            written_component_kinds.push(*component_kind);

            // place diff mask in a special transmission record - like map
            host_manager.last_update_packet_index = *packet_index;

            if !host_manager.sent_updates.contains_key(packet_index) {
                host_manager
                    .sent_updates
                    .insert(*packet_index, (now.clone(), HashMap::new()));
            }
            // SAFETY: We just ensured packet_index exists in the map with the contains_key/insert check above
            let (_, sent_updates_map) = host_manager.sent_updates.get_mut(packet_index).unwrap();
            sent_updates_map.insert((*entity, *component_kind), diff_mask);

            // having copied the diff mask for this update, clear the component
            host_manager
                .world_channel
                .diff_handler
                .clear_diff_mask(entity, component_kind);
        }

        // SAFETY: entity is from next_send_updates.keys(), so it must exist in the map
        let update_kinds = next_send_updates.get_mut(entity).unwrap();
        for component_kind in &written_component_kinds {
            update_kinds.remove(component_kind);
        }
        if update_kinds.is_empty() {
            next_send_updates.remove(entity);
        }

        Ok(())
    }

    fn create_update_overflow_error(
        component_kind: String,
        bits_needed: u32,
        bits_free: u32,
    ) -> WorldChannelError {
        WorldChannelError::ComponentUpdatePacketOverflow {
            component_kind,
            bits_needed,
            bits_free,
        }
    }
}
