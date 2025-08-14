use std::{net::SocketAddr, sync::RwLockReadGuard, hash::Hash, collections::{HashMap, VecDeque, HashSet}};

use naia_socket_shared::Instant;

use crate::{types::{HostType, PacketIndex}, world::{
    entity::{entity_converters::GlobalWorldManagerType, in_scope_entities::GlobalEntityReserver},
    host::{
        host_world_manager::{HostWorldManager, CommandId},
        entity_update_manager::EntityUpdateManager
    },
    remote::entity_waitlist::EntityWaitlist,
}, ComponentKind, ComponentKinds, ComponentUpdate, DiffMask, EntityAndGlobalEntityConverter, EntityCommand, EntityConverterMut, EntityEvent, EntityMessage, GlobalEntity, GlobalEntitySpawner, HostEntity, InScopeEntities, LocalEntityAndGlobalEntityConverter, LocalEntityMap, MessageIndex, PacketNotifiable, RemoteEntity, RemoteWorldManager, Replicate, Tick, WorldMutType, WorldRefType};

pub struct LocalWorldManager {
    entity_map: LocalEntityMap,
    host: HostWorldManager,
    remote: RemoteWorldManager,
    updater: EntityUpdateManager,
}

impl LocalWorldManager {

    pub fn entity_waitlist_mut(&mut self) -> &mut EntityWaitlist {
        self.remote.entity_waitlist_mut()
    }

    pub(crate) fn receive_message(&mut self, id: MessageIndex, msg: EntityMessage<RemoteEntity>) {
        self.remote.receive_message(id, msg);
    }

    pub(crate) fn insert_received_component(&mut self, remote_entity: &RemoteEntity, component_kind: &ComponentKind, component: Box<dyn Replicate>) {
        self.remote.insert_received_component(remote_entity, component_kind, component);
    }

    pub(crate) fn insert_received_update(
        &mut self,
        tick: Tick,
        global_entity: &GlobalEntity,
        component_update: ComponentUpdate
    ) {
        self.remote.insert_received_update(tick, global_entity, component_update);
    }

    pub(crate) fn contains_remote_entity(&self, remote_entity: &RemoteEntity) -> bool {
        self.entity_map.contains_remote_entity(remote_entity)
    }

    pub(crate) fn global_entity_from_remote(&self, remote_entity: &RemoteEntity) -> Option<&GlobalEntity> {
        self.entity_map.global_entity_from_remote(remote_entity)
    }

    pub(crate) fn remote_entities(&self) -> Vec<GlobalEntity> {
        self.entity_map.remote_entities()
    }

    pub fn process_received_commands(&mut self) {
        self.host.process_received_commands(
            &mut self.entity_map,
            &mut self.updater,
        );
    }

    pub fn take_update_events<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        &mut self,
        world: &W,
        world_converter: &dyn EntityAndGlobalEntityConverter<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
    ) -> HashMap<GlobalEntity, HashSet<ComponentKind>> {

        let mut updatable_world = self.host.get_updatable_world();
        let local_converter = self.entity_map.entity_converter();
        self.remote.append_updatable_world(local_converter, &mut updatable_world);
        self.updater.take_outgoing_events(world, world_converter, global_world_manager, updatable_world)
    }

    pub(crate) fn insert_sent_command_packet(&mut self, packet_index: &PacketIndex, now: Instant) {
        self.host.insert_sent_command_packet(packet_index, now);
    }

    pub(crate) fn record_command_written(
        &mut self,
        packet_index: &PacketIndex,
        command_id: &CommandId,
        message: EntityMessage<GlobalEntity>,
    ) {
        self.host.record_command_written(packet_index, command_id, message);
    }

    pub fn host_has_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.host.host_has_entity(global_entity)
    }

    pub(crate) fn get_diff_mask(&self, global_entity: &GlobalEntity, component_kind: &ComponentKind) -> RwLockReadGuard<'_, DiffMask> {
        self.updater.get_diff_mask(global_entity, component_kind)
    }

    pub(crate) fn record_update(
        &mut self,
        now: &Instant,
        packet_index: &PacketIndex,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
        diff_mask: DiffMask
    ) {
        self.updater.record_update(
            now,
            packet_index,
            global_entity,
            component_kind,
            diff_mask
        );
    }

    pub(crate) fn get_message_reader_helpers<'a, 'b, 'c, E: Copy + Eq + Hash + Sync + Send>(
        &'c mut self,
        global_entity_manager: &'a dyn GlobalWorldManagerType,
        spawner: &'b mut dyn GlobalEntitySpawner<E>
    ) -> (GlobalEntityReserver<'a, 'b, 'c, E>, &'c mut EntityWaitlist) {
        let reserver = self.entity_map.global_entity_reserver(global_entity_manager, spawner);
        let entity_waitlist = self.remote.entity_waitlist_mut();
        (reserver, entity_waitlist)
    }

    pub fn get_message_processor_helpers(&mut self) -> (&dyn LocalEntityAndGlobalEntityConverter, &mut EntityWaitlist) {
        let entity_converter = self.entity_map.entity_converter();
        let entity_waitlist = self.remote.entity_waitlist_mut();
        (entity_converter, entity_waitlist)
    }

    pub fn take_incoming_events(&mut self) -> Vec<EntityMessage<RemoteEntity>> {
        self.remote.take_incoming_events()
    }

    pub fn process_world_events<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        spawner: &mut dyn GlobalEntitySpawner<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        component_kinds: &ComponentKinds,
        world: &mut W,
        now: &Instant,
        incoming_messages: Vec<EntityMessage<RemoteEntity>>,
    ) -> Vec<EntityEvent> {
        self.remote.process_world_events(
            spawner,
            global_world_manager,
            &mut self.entity_map,
            component_kinds,
            world,
            now,
            incoming_messages,
        )
    }

    pub fn take_outgoing_events(
        &mut self,
        now: &Instant,
        rtt_millis: &f32,
    ) -> VecDeque<(CommandId, EntityCommand)> {
        self.host.take_outgoing_events(now, rtt_millis)
    }

    pub fn send_outgoing_command(
        &mut self,
        command: EntityCommand,
    ) {
        self.host.send_outgoing_command(command);
    }

    pub fn has_both_host_and_remote_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.entity_map.has_both_host_and_remote_entity(global_entity)
    }

    pub fn insert_with_remote_entity(&mut self, global_entity: GlobalEntity, remote: RemoteEntity) {
        self.entity_map.insert_with_remote_entity(global_entity, remote);
    }

    pub fn set_primary_to_host(&mut self, global_entity: &GlobalEntity) {
        self.entity_map.set_primary_to_host(global_entity);
    }

    pub fn host_init_entity(
        &mut self,
        global_entity: &GlobalEntity,
        component_kinds: Vec<ComponentKind>,
    ) {
        self.host.host_init_entity(global_entity, component_kinds);
    }

    pub fn host_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        self.host.host_despawn_entity(global_entity)
    }

    pub fn host_insert_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.host.host_insert_component(global_entity, component_kind);
    }

    pub fn host_remove_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.host.host_remove_component(global_entity, component_kind);
    }

    pub fn on_entity_channel_opened(
        &mut self,
        in_scope_entities: &dyn InScopeEntities,
        global_entity: &GlobalEntity
    ) {
        self.remote.on_entity_channel_opened(in_scope_entities, global_entity);
    }

    pub fn remote_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        self.host.remote_despawn_entity(global_entity);
    }

    pub fn on_remote_despawn_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        self.host.on_remote_despawn_entity(&mut self.entity_map, global_entity);
    }

    pub fn host_reserve_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) -> HostEntity {
        self.host.host_reserve_entity(&mut self.entity_map, global_entity)
    }

    pub fn remove_reserved_host_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) -> Option<HostEntity> {
        self.host.remove_reserved_host_entity(global_entity)
    }
}

impl LocalWorldManager {
    pub fn new(
        address: &Option<SocketAddr>,
        host_type: HostType,
        user_key: u64,
        global_world_manager: &dyn GlobalWorldManagerType,
    ) -> Self {
        Self {
            entity_map: LocalEntityMap::new(),
            host: HostWorldManager::new(host_type, user_key),
            remote: RemoteWorldManager::new(host_type),
            updater: EntityUpdateManager::new(address, global_world_manager),
        }
    }

    pub fn entity_converter(&self) -> &dyn LocalEntityAndGlobalEntityConverter {
        self.entity_map.entity_converter()
    }

    pub fn entity_converter_mut<'a, 'b>(
        &'b mut self,
        global_world_manager: &'a dyn GlobalWorldManagerType
    ) -> EntityConverterMut<'a, 'b> {
        self.host.entity_converter_mut(
            global_world_manager,
            &mut self.entity_map,
        )
    }

    pub fn collect_messages(&mut self, now: &Instant, rtt_millis: &f32) {
        self.host
            .handle_dropped_command_packets(now);
        self.updater
            .handle_dropped_update_packets(now, rtt_millis);
    }
}

impl PacketNotifiable for LocalWorldManager {
    fn notify_packet_delivered(&mut self, packet_index: PacketIndex) {
        self.host.notify_packet_delivered(packet_index);
        self.updater.notify_packet_delivered(packet_index);
    }
}