use std::{hash::Hash, collections::{HashMap, HashSet}};

use log::info;

use crate::{messages::channels::receivers::reliable_receiver::ReliableReceiver, world::{
    sync::{HostEngine, RemoteEngine, EntityChannelReceiver, EntityChannelSender},
    host::entity_update_manager::EntityUpdateManager,
}, EntityMessage, EntityMessageReceiver, GlobalEntity, HostType, ComponentKind, HostEntityGenerator, MessageIndex, EntityCommand, LocalEntityMap, HostEntity, EntityConverterMut, GlobalWorldManagerType, ShortMessageIndex, WorldMutType, GlobalEntitySpawner, EntityEvent, LocalEntityAndGlobalEntityConverter, OwnedLocalEntity};

pub type CommandId = MessageIndex;
pub type SubCommandId = ShortMessageIndex;

/// Channel to perform ECS replication between server and client
/// Only handles entity commands (Spawn/despawn entity and insert/remove components)
/// Will use a reliable sender.
/// Will wait for acks from the client to know the state of the client's ECS world ("remote")
pub struct HostWorldManager {

    // host entity generator
    entity_generator: HostEntityGenerator,

    // For Server, this contains the Entities that the Server has authority over, that it syncs to the Client
    // For Client, this contains the non-Delegated Entities that the Client has authority over, that it syncs to the Server
    host_engine: HostEngine,

    // For Server, this contains the Entities that the Server has authority over, that have been delivered to the Client
    // For Client, this contains the non-Delegated Entities that the Client has authority over, that have been delivered to the Server
    delivered_receiver: ReliableReceiver<EntityMessage<HostEntity>>,
    delivered_engine: RemoteEngine<HostEntity>,
    incoming_events: Vec<EntityEvent>
}

impl HostWorldManager {
    pub fn new(host_type: HostType, user_key: u64) -> Self {
        Self {
            entity_generator: HostEntityGenerator::new(user_key),
            host_engine: HostEngine::new(host_type),
            delivered_receiver: ReliableReceiver::new(),
            delivered_engine: RemoteEngine::new(host_type.invert()),
            incoming_events: Vec::new(),
        }
    }

    pub(crate) fn entity_converter_mut<'a, 'b>(
        &'b mut self,
        global_world_manager: &'a dyn GlobalWorldManagerType,
        entity_map: &'b mut LocalEntityMap,
    ) -> EntityConverterMut<'a, 'b> {
        EntityConverterMut::new(global_world_manager, entity_map, &mut self.entity_generator)
    }

    // Collect

    pub fn take_incoming_events<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        spawner: &mut dyn GlobalEntitySpawner<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        local_entity_map: &mut LocalEntityMap,
        world: &mut W,
        incoming_messages: Vec<(MessageIndex, EntityMessage<HostEntity>)>,
    ) -> Vec<EntityEvent> {

        let incoming_messages = EntityMessageReceiver::host_take_incoming_events(
            &mut self.host_engine,
            incoming_messages,
        );

        self.process_incoming_messages(
            spawner,
            global_world_manager,
            local_entity_map,
            world,
            incoming_messages,
        );

        std::mem::take(&mut self.incoming_events)
    }

    pub fn take_outgoing_commands(
        &mut self,
    ) -> Vec<EntityCommand> {
        self.host_engine.take_outgoing_commands()
    }

    pub fn send_command(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        command: EntityCommand,
    ) {
        self.host_engine.send_command(converter, command);
    }

    pub(crate) fn host_generate_entity(&mut self) -> HostEntity {
        self.entity_generator.generate_host_entity()
    }
    
    pub(crate) fn host_reserve_entity(
        &mut self,
        entity_map: &mut LocalEntityMap,
        global_entity: &GlobalEntity,
    ) -> HostEntity {
        self.entity_generator.host_reserve_entity(entity_map, global_entity)
    }

    pub(crate) fn host_removed_reserved_entity(
        &mut self,
        global_entity: &GlobalEntity,
    ) -> Option<HostEntity> {
        self.entity_generator.host_remove_reserved_entity(global_entity)
    }

    pub(crate) fn host_has_entity(&self, host_entity: &HostEntity) -> bool {
        self.get_host_world().contains_key(host_entity)
    }

    // used when Entity first comes into Connection's scope
    pub fn host_init_entity(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        global_entity: &GlobalEntity,
        component_kinds: Vec<ComponentKind>,
    ) {
        // add entity
        self.host_spawn_entity(converter, global_entity);
        // add components
        for component_kind in component_kinds {
            self.host_insert_component(converter, global_entity, &component_kind);
        }
    }

    fn host_spawn_entity(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        global_entity: &GlobalEntity,
    ) {
        self.host_engine.send_command(converter, EntityCommand::Spawn(*global_entity));
    }

    pub fn host_despawn_entity(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        global_entity: &GlobalEntity
    ) {
        self.host_engine.send_command(converter, EntityCommand::Despawn(*global_entity));
    }

    pub fn host_insert_component(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.host_engine.send_command(converter, EntityCommand::InsertComponent(*global_entity, *component_kind));
    }

    pub fn host_remove_component(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        self.host_engine.send_command(converter, EntityCommand::RemoveComponent(*global_entity, *component_kind));
    }

    pub fn remote_despawn_entity(&mut self, _global_entity: &GlobalEntity) {
        todo!("close entity channel?");
    }

    pub(crate) fn get_host_world(&self) -> &HashMap<HostEntity, EntityChannelSender> {
        self.host_engine.get_world()
    }

    pub(crate) fn get_remote_world(&self) -> &HashMap<HostEntity, EntityChannelReceiver> {
        self.delivered_engine.get_world()
    }

    pub(crate) fn get_updatable_world(&self, converter: &dyn LocalEntityAndGlobalEntityConverter) -> HashMap<GlobalEntity, HashSet<ComponentKind>> {
        let mut output = HashMap::new();
        for (host_entity, host_channel) in self.get_host_world() {

            let Some(remote_channel) = self.get_remote_world().get(host_entity) else {
                continue;
            };
            let host_component_kinds = host_channel.component_kinds();
            let joined_component_kinds = remote_channel.component_kinds_intersection(host_component_kinds);
            if joined_component_kinds.is_empty() {
                continue;
            }
            
            let global_entity = converter.host_entity_to_global_entity(host_entity).expect("Host entity not found in local entity map");
            output.insert(global_entity, joined_component_kinds);
        }
        output
    }

    pub(crate) fn deliver_message(&mut self, command_id: CommandId, message: EntityMessage<OwnedLocalEntity>) {
        let Some(local_entity) = message.entity() else {
            return;
        };
        if local_entity.is_remote() {
            return;
        }
        let host_entity = local_entity.host();
        let host_message = message.with_entity(host_entity);
        self.delivered_receiver.buffer_message(command_id, host_message);
    }

    pub(crate) fn process_delivered_commands(
        &mut self,
        local_entity_map: &mut LocalEntityMap,
        entity_update_manager: &mut EntityUpdateManager,
    ) {
        let delivered_messages: Vec<(MessageIndex, EntityMessage<HostEntity>)> = self.delivered_receiver.receive_messages();
        for message in EntityMessageReceiver::remote_take_incoming_messages(
            &mut self.delivered_engine, 
            delivered_messages
        ) {
            match message {
                EntityMessage::Spawn(host_entity) => {
                    self.on_remote_spawn_entity(&host_entity);
                }
                EntityMessage::Despawn(host_entity) => {
                    self.on_remote_despawn_host_entity(local_entity_map, &host_entity);
                }
                EntityMessage::InsertComponent(host_entity, component_kind) => {
                    let global_entity = local_entity_map.global_entity_from_host(&host_entity).expect("Host entity not found in local entity map");
                    self.on_remote_insert_component(entity_update_manager, global_entity, &component_kind);
                }
                EntityMessage::RemoveComponent(host_entity, component_kind) => {
                    let global_entity = local_entity_map.global_entity_from_host(&host_entity).expect("Host entity not found in local entity map");
                    self.on_remote_remove_component(entity_update_manager, global_entity, &component_kind);
                }
                EntityMessage::Noop => {
                    // do nothing
                }
                _ => {
                    // Only Auth-related messages are left here
                    // Right now it doesn't seem like we need to track auth state here
                }
            }
        }
    }

    fn process_incoming_messages<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        _spawner: &mut dyn GlobalEntitySpawner<E>,
        _global_world_manager: &dyn GlobalWorldManagerType,
        _local_entity_map: &mut LocalEntityMap,
        _world: &mut W,
        incoming_messages: Vec<EntityMessage<HostEntity>>,
    ) {
        // execute the action and emit an event
        for message in incoming_messages {
            info!("Processing EntityMessage<HostEntity>: {:?}", message);
            match message {
                EntityMessage::Spawn(_) => {
                    todo!("Implement EntityMessage::<HostEntity>::Spawn handling");
                }
                EntityMessage::Despawn(_) => {
                    todo!("Implement EntityMessage::<HostEntity>::Despawn handling");
                }
                EntityMessage::InsertComponent(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::InsertComponent handling");
                }
                EntityMessage::RemoveComponent(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::RemoveComponent handling");
                }
                EntityMessage::Publish(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::Publish handling");
                }
                EntityMessage::Unpublish(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::Unpublish handling");
                }
                EntityMessage::EnableDelegation(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::EnableDelegation handling");
                }
                EntityMessage::DisableDelegation(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::DisableDelegation handling");
                }
                EntityMessage::SetAuthority(_, _, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::SetAuthority handling");
                }
                EntityMessage::RequestAuthority(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::RequestAuthority handling");
                }
                EntityMessage::ReleaseAuthority(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::ReleaseAuthority handling");
                }
                EntityMessage::EnableDelegationResponse(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::EnableDelegationResponse handling");
                }
                EntityMessage::MigrateResponse(_, _, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::MigrateResponse handling");
                }
                EntityMessage::Noop => {
                    // do nothing
                }
            }
        }
    }

    fn on_remote_spawn_entity(
        &mut self,
        _host_entity: &HostEntity,
    ) {
        // stubbed
    }

    pub fn on_remote_despawn_global_entity(
        &mut self,
        local_entity_map: &mut LocalEntityMap,
        global_entity: &GlobalEntity,
    ) {
        self.entity_generator.remove_by_global_entity(local_entity_map, global_entity);
    }

    pub fn on_remote_despawn_host_entity(
        &mut self,
        local_entity_map: &mut LocalEntityMap,
        host_entity: &HostEntity,
    ) {
        self.entity_generator.remove_by_host_entity(local_entity_map, host_entity);
    }

    fn on_remote_insert_component(
        &mut self,
        entity_update_manager: &mut EntityUpdateManager,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        entity_update_manager.register_component(global_entity, component_kind);
    }

    fn on_remote_remove_component(
        &mut self,
        entity_update_manager: &mut EntityUpdateManager,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        entity_update_manager.deregister_component(global_entity, component_kind);
    }
}