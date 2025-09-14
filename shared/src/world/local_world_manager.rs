use std::{time::Duration, net::SocketAddr, sync::RwLockReadGuard, hash::Hash, collections::{HashMap, VecDeque, HashSet}};

use naia_socket_shared::Instant;

use crate::{
    sequence_list::SequenceList,
    messages::channels::receivers::reliable_receiver::ReliableReceiver, types::{HostType, PacketIndex}, world::{
    entity::entity_converters::GlobalWorldManagerType,
    host::{
        host_world_manager::{HostWorldManager, CommandId},
        entity_update_manager::EntityUpdateManager
    },
    remote::entity_waitlist::{EntityWaitlist, WaitlistStore},
}, ChannelSender, ComponentKind, ComponentKinds, ComponentUpdate, DiffMask, EntityAndGlobalEntityConverter, EntityAuthStatus, EntityCommand, EntityConverterMut, EntityEvent, EntityMessage, EntityMessageType, GlobalEntity, GlobalEntitySpawner, HostEntity, LocalEntityAndGlobalEntityConverter, LocalEntityMap, MessageIndex, OwnedLocalEntity, PacketNotifiable, ReliableSender, RemoteEntity, RemoteWorldManager, Replicate, Tick, WorldMutType, WorldRefType};

const RESEND_COMMAND_RTT_FACTOR: f32 = 1.5;
const COMMAND_RECORD_TTL: Duration = Duration::from_secs(60);

pub struct LocalWorldManager {
    entity_map: LocalEntityMap,
    sender: ReliableSender<EntityCommand>,
    sent_command_packets: SequenceList<(Instant, Vec<(CommandId, EntityMessage<OwnedLocalEntity>)>)>,
    receiver: ReliableReceiver<EntityMessage<OwnedLocalEntity>>,

    host: HostWorldManager,
    remote: RemoteWorldManager,
    updater: EntityUpdateManager,
}

impl LocalWorldManager {
    pub(crate) fn entity_waitlist_queue<T>(
        &mut self,
        remote_entity_set: &HashSet<RemoteEntity>,
        waitlist_store: &mut WaitlistStore<T>,
        message: T,
    ) {
        self.remote.entity_waitlist_queue(
            remote_entity_set,
            waitlist_store,
            message,
        );
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
            entity_map: LocalEntityMap::new(host_type),
            sender: ReliableSender::new(RESEND_COMMAND_RTT_FACTOR),
            sent_command_packets: SequenceList::new(),
            receiver: ReliableReceiver::new(),

            host: HostWorldManager::new(host_type, user_key),
            remote: RemoteWorldManager::new(host_type),
            updater: EntityUpdateManager::new(address, global_world_manager),
        }
    }

    // EntityMap-focused

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

    pub(crate) fn contains_remote_entity(&self, remote_entity: &RemoteEntity) -> bool {
        self.entity_map.contains_remote_entity(remote_entity)
    }

    pub fn remote_entities(&self) -> Vec<GlobalEntity> {
        self.entity_map.remote_entities()
    }

    pub fn insert_with_remote_entity(&mut self, global_entity: GlobalEntity, remote: RemoteEntity) {
        self.entity_map.insert_with_remote_entity(global_entity, remote);
    }

    pub(crate) fn global_entity_from_remote(&self, remote_entity: &RemoteEntity) -> Option<&GlobalEntity> {
        self.entity_map.global_entity_from_remote(remote_entity)
    }

    pub fn has_both_host_and_remote_entity(&self, global_entity: &GlobalEntity) -> bool {
        self.entity_map.has_both_host_and_remote_entity(global_entity)
    }

    pub fn set_host_owned(&mut self, global_entity: &GlobalEntity) {
        self.entity_map.set_host_owned(global_entity);
    }

    // Host-focused

    pub fn host_has_entity(&self, global_entity: &GlobalEntity) -> bool {
        let Ok(host_entity) = self.entity_map.global_entity_to_host_entity(global_entity) else {
            return false;
        };
        self.host.host_has_entity(&host_entity)
    }

    pub fn host_init_entity(
        &mut self,
        global_entity: &GlobalEntity,
        component_kinds: Vec<ComponentKind>,
    ) {
        if self.entity_map.global_entity_to_host_entity(global_entity).is_err() {
            let host_entity = self.host.host_generate_entity();
            self.entity_map.insert_with_host_entity(*global_entity, host_entity);
        }
        self.host.host_init_entity(&self.entity_map, global_entity, component_kinds);
    }

    pub fn host_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        self.host.host_despawn_entity(&self.entity_map, global_entity)
    }

    // should be remote? or maybe not, this is after migration? only server sends this
    pub fn host_send_migrate_response(
        &mut self,
        _global_entity: &GlobalEntity,
    ) {
        todo!();

        // Add remote entity to Host World
        // let new_host_entity = connection.base.host_world_manager.track_remote_entity(
        //     &mut connection.base.local_world_manager,
        //     global_entity,
        //     component_kinds,
        // );

        // let command = EntityCommand::MigrateResponse(None, *global_entity, new_host_entity);
        // self.host.send_command(&self.entity_map, command);
    }

    pub fn host_send_set_auth(
        &mut self,
        global_entity: &GlobalEntity,
        auth_status: EntityAuthStatus,
    ) {
        // TODO: ?
        let command = EntityCommand::SetAuthority(None, *global_entity, auth_status);
        self.host.send_command(&self.entity_map, command);
    }

    pub fn host_reserve_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) -> HostEntity {
        self.host.host_reserve_entity(&mut self.entity_map, global_entity)
    }

    pub fn host_remove_reserved_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) -> Option<HostEntity> {
        self.host.host_removed_reserved_entity(global_entity)
    }

    pub fn on_remote_despawn_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        self.host.remote_despawn_entity(global_entity);
        self.host.on_remote_despawn_global_entity(&mut self.entity_map, global_entity);
    }

    pub(crate) fn insert_sent_command_packet(&mut self, packet_index: &PacketIndex, now: Instant) {
        if !self
            .sent_command_packets
            .contains_scan_from_back(packet_index)
        {
            self
                .sent_command_packets
                .insert_scan_from_back(*packet_index, (now, Vec::new()));
        }
    }

    pub(crate) fn record_command_written(
        &mut self,
        packet_index: &PacketIndex,
        command_id: &CommandId,
        message: EntityMessage<OwnedLocalEntity>,
    ) {
        let (_, sent_actions_list) = self.sent_command_packets.get_mut_scan_from_back(packet_index).unwrap();
        sent_actions_list.push((*command_id, message));
    }

    // Remote-focused

    // only client sends this, after receiving enabledelegation message from server
    pub fn send_enable_delegation_response(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        let command = EntityCommand::EnableDelegationResponse(None, *global_entity);
        self.remote.send_command(&self.entity_map, command);
    }

    pub fn remote_send_request_auth(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        let new_host_entity = self.host_reserve_entity(&global_entity); // host entity? on remote? this is wrong
        let command = EntityCommand::RequestAuthority(None, *global_entity, new_host_entity);
        self.remote.send_command(&self.entity_map, command);
    }

    pub fn remote_send_release_auth(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        let command = EntityCommand::ReleaseAuthority(None, *global_entity);
        self.remote.send_command(&self.entity_map, command);
    }

    pub fn entity_waitlist_mut(&mut self) -> &mut EntityWaitlist<RemoteEntity> {
        self.remote.entity_waitlist_mut()
    }

    pub(crate) fn receiver_buffer_message(&mut self, id: MessageIndex, msg: EntityMessage<OwnedLocalEntity>) {
        self.receiver.buffer_message(id, msg);
    }

    pub(crate) fn insert_received_component(&mut self, local_entity: &OwnedLocalEntity, component_kind: &ComponentKind, component: Box<dyn Replicate>) {
        self.remote.insert_received_component(local_entity, component_kind, component);
    }

    pub(crate) fn insert_received_update(
        &mut self,
        tick: Tick,
        global_entity: &GlobalEntity,
        component_update: ComponentUpdate
    ) {
        let remote_entity = self.entity_map.global_entity_to_remote_entity(global_entity).unwrap();
        self.remote.insert_received_update(tick, &remote_entity, component_update);
    }

    pub fn take_incoming_events<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        spawner: &mut dyn GlobalEntitySpawner<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        component_kinds: &ComponentKinds,
        world: &mut W,
        now: &Instant,
    ) -> Vec<EntityEvent> {
        let incoming_messages = self.receiver.receive_messages();
        let mut incoming_host_messages = Vec::new();
        let mut incoming_remote_messages = Vec::new();
        
        for (id, incoming_message) in incoming_messages {
            if incoming_message.get_type() == EntityMessageType::Noop {
                continue; // skip noop messages
            }
            let Some(local_entity) = incoming_message.entity() else {
                panic!("Received message without an entity! Message: {:?}", incoming_message);
            };
            match local_entity {
                OwnedLocalEntity::Host(host_entity) => {
                    // Host entity message
                    let host_entity = HostEntity::new(host_entity);
                    incoming_host_messages.push((id, incoming_message.with_entity(host_entity)));
                }
                OwnedLocalEntity::Remote(remote_entity) => {
                    // Remote entity message
                    let remote_entity = RemoteEntity::new(remote_entity);
                    incoming_remote_messages.push((id, incoming_message.with_entity(remote_entity)));
                }
            }
        }
        
        let host_events = self.host.take_incoming_events(
            spawner,
            global_world_manager,
            &mut self.entity_map,
            world,
            incoming_host_messages,
        );
        let mut remote_events = self.remote.take_incoming_events(
            spawner,
            global_world_manager,
            &mut self.entity_map,
            component_kinds,
            world,
            now,
            incoming_remote_messages
        );

        let mut incoming_events = host_events;
        incoming_events.append(&mut remote_events);

        incoming_events
    }

    pub fn on_entity_channel_opened(
        &mut self,
        global_entity: &GlobalEntity
    ) {
        let remote_entity = self.entity_map.global_entity_to_remote_entity(global_entity).unwrap();
        self.remote.on_entity_channel_opened(&remote_entity);
    }

    // Update-focused

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

    // Joint router

    // todo: should work on delegated client-owned entities too
    pub fn host_insert_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.host.host_insert_component(&self.entity_map, global_entity, component_kind);
    }

    // todo: should work on delegated client-owned entities too
    pub fn host_remove_component(
        &mut self,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.host.host_remove_component(&self.entity_map, global_entity, component_kind);
    }

    pub fn send_publish(
        &mut self,
        host_type: HostType,
        global_entity: &GlobalEntity,
    ) {
        let Ok(local_entity) = self.entity_map.global_entity_to_owned_entity(global_entity) else {
            panic!("Attempting to publish entity which does not exist in local entity map! {:?}", global_entity);
        };
        let host_owned = match (host_type, local_entity.is_host()) {
            (HostType::Server, true) => panic!("Server-owned Entities are published by default, invalid!"),
            (HostType::Client, false) => panic!("Server-owned Entities are published by default, invalid!"),
            (HostType::Server, false) => false, // todo!("server is attempting to publish a client-owned non-public remote entity"),
            (HostType::Client, true) => true, // todo!("client is attempting to publish a client-owned host entity"),
        };

        let command = EntityCommand::Publish(None, *global_entity);
        if host_owned {
            self.host.send_command(&self.entity_map, command);
        } else {
            self.remote.send_command(self.entity_map.entity_converter(), command);
        }
    }

    pub fn send_unpublish(
        &mut self,
        host_type: HostType,
        global_entity: &GlobalEntity,
    ) {
        let Ok(local_entity) = self.entity_map.global_entity_to_owned_entity(global_entity) else {
            panic!("Attempting to publish entity which does not exist in local entity map! {:?}", global_entity);
        };
        let host_owned = match (host_type, local_entity.is_host()) {
            (HostType::Server, true) => panic!("Server-owned Entities cannot be unpublished!"),
            (HostType::Client, false) => panic!("Server-owned Entities cannot be unpublished!"),
            (HostType::Server, false) => false, // todo!("server is attempting to unpublish a client-owned public entity"),
            (HostType::Client, true) => true, // todo!("client is attempting to unpublish a client-owned public entity"),
        };
        let command = EntityCommand::Unpublish(None, *global_entity);
        if host_owned {
            self.host.send_command(&self.entity_map, command);
        } else {
            self.remote.send_command(self.entity_map.entity_converter(), command);
        }
    }

    pub fn send_enable_delegation(
        &mut self,
        host_type: HostType,
        origin_is_owning_client: bool,
        global_entity: &GlobalEntity,
    ) {
        let is_delegated = self.entity_map.global_entity_is_delegated(global_entity);
        if is_delegated {
            panic!("Entity {:?} is already delegated!", global_entity);
        }
        let Ok(local_entity) = self.entity_map.global_entity_to_owned_entity(global_entity) else {
            panic!("Attempting to enable delegation for entity which does not exist in local entity map! {:?}", global_entity);
        };
        let host_owned = match (host_type, local_entity.is_host(), origin_is_owning_client) {
            (HostType::Server, false, true) => panic!("Client cannot originate enable delegation for ANOTHER client-owned entity!"),
            (HostType::Client, _, false) => panic!("Client must be the owning client to enable delegation!"),
            (HostType::Client, false, true) => panic!("Client cannot enable delegation for a Server-owned entity"),

            (HostType::Server, true, true) => true,    // todo!("server is proxying client-originating enable delegation message to client (entity should be host-owned here)"),
            (HostType::Server, true, false) => true,   // todo!("server is enabling delegation for a server-owned entity (host owned)"),
            (HostType::Client, true, true) => true,    // todo!("client is attempting to enable delegation for a client-owned entity (host owned)"),
            (HostType::Server, false, false) => false, // todo!("server is attempting to delegate a (hopefully published) client-owned entity (remote-owned entity"),
        };

        let command = EntityCommand::EnableDelegation(None, *global_entity);
        if host_owned {
            self.host.send_command(&self.entity_map, command);
        } else {
            self.remote.send_command(self.entity_map.entity_converter(), command);
        }
    }

    pub fn send_disable_delegation(
        &mut self,
        global_entity: &GlobalEntity,
    ) {
        // only server should ever be able to call this, on host-owned (server-owned) entities
        let command = EntityCommand::DisableDelegation(None, *global_entity);
        self.host.send_command(&self.entity_map, command);
    }

    pub fn track_hosts_redundant_remote_entity(
        &mut self,
        _remote_entity: &RemoteEntity,
        _component_kinds: &Vec<ComponentKind>,
    ) {
        todo!();
    }

    pub fn untrack_hosts_redundant_remote_entity(
        &mut self,
        _remote_entity: &RemoteEntity
    ) {
        todo!();
    }

    // Joint

    pub fn collect_messages(&mut self, now: &Instant, rtt_millis: &f32) {
        self.handle_dropped_command_packets(now);
        self.updater.handle_dropped_update_packets(now, rtt_millis);
    }

    fn handle_dropped_command_packets(&mut self, now: &Instant) {
        let mut pop = false;

        loop {
            if let Some((_, (time_sent, _))) = self.sent_command_packets.front() {
                if time_sent.elapsed(now) > COMMAND_RECORD_TTL {
                    pop = true;
                }
            } else {
                return;
            }
            if pop {
                self.sent_command_packets.pop_front();
            } else {
                return;
            }
        }
    }

    pub fn take_outgoing_events<E: Copy + Eq + Hash + Send + Sync, W: WorldRefType<E>>(
        &mut self,
        now: &Instant,
        rtt_millis: &f32,
        world: &W,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
    ) -> (VecDeque<(CommandId, EntityCommand)>, HashMap<GlobalEntity, HashSet<ComponentKind>>) {

        // get outgoing world commands
        let host_commands = self.host.take_outgoing_commands();
        let remote_commands = self.remote.take_outgoing_commands();
        for commands in [host_commands, remote_commands] {
            for command in commands {
                self.sender.send_message(command);
            }
        }
        self.sender.collect_messages(now, rtt_millis);
        let world_commands = self.sender.take_next_messages();

        // get update events
        let update_events = self.take_update_events(world, converter, global_world_manager);

        // return both
        (world_commands, update_events)
    }

    pub fn process_delivered_commands(&mut self) {
        self.host.process_delivered_commands(
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

        let mut updatable_world = self.host.get_updatable_world(&self.entity_map);
        let local_converter = self.entity_map.entity_converter();
        self.remote.append_updatable_world(local_converter, &mut updatable_world);
        self.updater.take_outgoing_events(world, world_converter, global_world_manager, updatable_world)
    }

    // pub(crate) fn get_message_reader_helpers<'a, 'b, 'c, E: Copy + Eq + Hash + Sync + Send>(
    //     &'b mut self,
    //     spawner: &'b mut dyn GlobalEntitySpawner<E>
    // ) -> (GlobalEntityReserver<'a, 'b, 'c, E>, &'a mut EntityWaitlist<RemoteEntity>) {
    //     let remote= &mut self.remote;
    //     let entity_map = &mut self.entity_map;
    //     let reserver = remote.get_message_reader_helpers(entity_map, spawner);
    //     (reserver, remote.entity_waitlist_mut())
    // }

    pub fn get_message_processor_helpers(&mut self) -> (&dyn LocalEntityAndGlobalEntityConverter, &mut EntityWaitlist<RemoteEntity>) {
        let entity_converter = self.entity_map.entity_converter();
        let entity_waitlist = self.remote.entity_waitlist_mut();
        (entity_converter, entity_waitlist)
    }

    fn host_notify_packet_delivered(&mut self, packet_index: PacketIndex) {
        if let Some((_, command_list)) = self
            .sent_command_packets
            .remove_scan_from_front(&packet_index)
        {
            for (command_id, command) in command_list {
                if self.sender.deliver_message(&command_id).is_some() {
                    self.host.deliver_message(command_id, command);
                }
            }
        }
    }
}

impl PacketNotifiable for LocalWorldManager {
    fn notify_packet_delivered(&mut self, packet_index: PacketIndex) {
        self.host_notify_packet_delivered(packet_index);
        self.updater.notify_packet_delivered(packet_index);
    }
}