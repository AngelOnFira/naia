use std::{collections::{HashMap, HashSet}, hash::Hash};

use log::info;

use crate::{messages::channels::receivers::reliable_receiver::ReliableReceiver, world::sync::{EntityChannelReceiver, EntityChannelSender, HostEngine, RemoteEngine}, ComponentKind, EntityCommand, EntityConverterMut, EntityEvent, EntityMessage, EntityMessageReceiver, GlobalEntity, GlobalEntitySpawner, GlobalWorldManagerType, HostEntity, HostEntityGenerator, HostType, LocalEntityAndGlobalEntityConverter, LocalEntityMap, MessageIndex, ShortMessageIndex, WorldMutType};
use crate::world::update::entity_update_manager::EntityUpdateManager;

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
        local_entity_map: &LocalEntityMap,
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

    pub(crate) fn has_entity(&self, host_entity: &HostEntity) -> bool {
        self.get_host_world().contains_key(host_entity)
    }

    // used when Entity first comes into Connection's scope
    pub fn init_entity(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        global_entity: &GlobalEntity,
        component_kinds: Vec<ComponentKind>,
    ) {
        // add entity
        self.host_engine.send_command(converter, EntityCommand::Spawn(*global_entity));
        // add components
        for component_kind in component_kinds {
            self.host_engine.send_command(converter, EntityCommand::InsertComponent(*global_entity, component_kind));
        }
    }

    pub fn send_command(
        &mut self,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        command: EntityCommand,
    ) {
        self.host_engine.send_command(converter, command);
    }

    pub(crate) fn get_host_world(&self) -> &HashMap<HostEntity, EntityChannelSender> {
        self.host_engine.get_world()
    }

    pub(crate) fn get_delivered_world(&self) -> &HashMap<HostEntity, EntityChannelReceiver> {
        self.delivered_engine.get_world()
    }

    pub(crate) fn get_updatable_world(&self, converter: &dyn LocalEntityAndGlobalEntityConverter) -> HashMap<GlobalEntity, HashSet<ComponentKind>> {
        let mut output = HashMap::new();
        for (host_entity, host_channel) in self.get_host_world() {

            let Some(delivered_channel) = self.get_delivered_world().get(host_entity) else {
                continue;
            };
            let host_component_kinds = host_channel.component_kinds();
            let joined_component_kinds = delivered_channel.component_kinds_intersection(host_component_kinds);
            if joined_component_kinds.is_empty() {
                continue;
            }
            
            let global_entity = converter.host_entity_to_global_entity(host_entity).expect("Host entity not found in local entity map");
            output.insert(global_entity, joined_component_kinds);
        }
        output
    }

    pub(crate) fn deliver_message(&mut self, command_id: CommandId, message: EntityMessage<HostEntity>) {
        self.delivered_receiver.buffer_message(command_id, message);
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
                    self.on_delivered_spawn_entity(&host_entity);
                }
                EntityMessage::Despawn(host_entity) => {
                    self.on_delivered_despawn_entity(local_entity_map, &host_entity);
                }
                EntityMessage::InsertComponent(host_entity, component_kind) => {
                    let global_entity = local_entity_map.global_entity_from_host(&host_entity).expect("Host entity not found in local entity map");
                    self.on_delivered_insert_component(entity_update_manager, global_entity, &component_kind);
                }
                EntityMessage::RemoveComponent(host_entity, component_kind) => {
                    let global_entity = local_entity_map.global_entity_from_host(&host_entity).expect("Host entity not found in local entity map");
                    self.on_delivered_remove_component(entity_update_manager, global_entity, &component_kind);
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
        local_entity_map: &LocalEntityMap,
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
                EntityMessage::ReleaseAuthority(_, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::ReleaseAuthority handling");
                }
                EntityMessage::MigrateResponse(_, _, _) => {
                    todo!("Implement EntityMessage::<HostEntity>::MigrateResponse handling");
                }
                EntityMessage::Noop => {
                    // do nothing
                }
                // Whitelisted incoming messages:
                // 1. EntityMessage::EnableDelegationResponse
                // 2. EntityMessage::RequestAuthority
                msg => {
                    let event = msg.to_event(local_entity_map);
                    self.incoming_events.push(event);
                }
            }
        }
    }

    fn on_delivered_spawn_entity(
        &mut self,
        _host_entity: &HostEntity,
    ) {
        // stubbed
    }

    pub fn on_delivered_despawn_entity(
        &mut self,
        local_entity_map: &mut LocalEntityMap,
        host_entity: &HostEntity,
    ) {
        self.entity_generator.remove_by_host_entity(local_entity_map, host_entity);
    }

    fn on_delivered_insert_component(
        &mut self,
        entity_update_manager: &mut EntityUpdateManager,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        entity_update_manager.register_component(global_entity, component_kind);
    }

    fn on_delivered_remove_component(
        &mut self,
        entity_update_manager: &mut EntityUpdateManager,
        global_entity: &GlobalEntity,
        component_kind: &ComponentKind,
    ) {
        entity_update_manager.deregister_component(global_entity, component_kind);
    }
}