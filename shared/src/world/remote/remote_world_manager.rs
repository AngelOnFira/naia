use std::{
    collections::HashMap,
    hash::Hash,
};

use log::warn;

use naia_socket_shared::Instant;

use crate::{world::{
    entity::{local_entity::RemoteEntity, in_scope_entities::{InScopeEntities, InScopeEntitiesMut}},
    remote::{
        remote_world_waitlist::RemoteWorldWaitlist,
        entity_event::EntityEvent,
        entity_waitlist::EntityWaitlist,
    },
    sync::{EntityChannelReceiver, EntityChannelSender, SenderEngine},
}, ComponentKind, ComponentKinds, ComponentUpdate, EntityMessage, EntityAndGlobalEntityConverter, GlobalEntity, GlobalEntitySpawner, GlobalWorldManagerType, LocalEntityAndGlobalEntityConverter, Replicate, Tick, WorldMutType, EntityMessageType, OwnedLocalEntity, HostEntity, EntityMessageReceiver, HostType, MessageIndex, LocalEntityMap};

pub struct RemoteWorldManager {
    
    // For Server, this contains the Entities that have been received from the Client, that the Client has authority over.
    // For Client, this contains the Entities that have been received from the Server, that the Server has authority over.
    receiver: EntityMessageReceiver<RemoteEntity>,

    // For Server, this will be set to None, as the Server does not delegate authority to the Client.
    // For Client, this contains the Delegated Entities that the Client now has authority over.
    delegated_world_opt: Option<SenderEngine>,
    
    // incoming messages
    incoming_components: HashMap<(RemoteEntity, ComponentKind), Box<dyn Replicate>>,
    incoming_updates: Vec<(Tick, GlobalEntity, ComponentUpdate)>,
    incoming_events: Vec<EntityEvent>,
    waitlist: RemoteWorldWaitlist,
}

impl RemoteWorldManager {
    pub fn new(host_type: HostType) -> Self {
        let delegated_world_opt = if host_type == HostType::Client {
            Some(SenderEngine::new(host_type))
        } else {
            None
        };
        Self {
            receiver: EntityMessageReceiver::new(host_type),
            incoming_components: HashMap::new(),
            incoming_updates: Vec::new(),
            incoming_events: Vec::new(),
            waitlist: RemoteWorldWaitlist::new(),
            delegated_world_opt,
        }
    }

    pub fn entity_waitlist(&self) -> &EntityWaitlist {
        self.waitlist.entity_waitlist()
    }

    pub fn entity_waitlist_mut(&mut self) -> &mut EntityWaitlist {
        self.waitlist.entity_waitlist_mut()
    }

    pub fn get_host_world(&self) -> Option<&HashMap<GlobalEntity, EntityChannelSender>> {
        self.delegated_world_opt.as_ref().map(|engine| engine.get_world())
    }

    pub fn get_remote_world(&self) -> &HashMap<RemoteEntity, EntityChannelReceiver> {
        self.receiver.get_world()
    }

    pub fn receive_message(&mut self, message_id: MessageIndex, message: EntityMessage<RemoteEntity>) {
        self.receiver.buffer_message(message_id, message);
    }

    pub fn take_incoming_events(&mut self) -> Vec<EntityMessage<RemoteEntity>> {
        self.receiver.receive_messages()
    }

    pub(crate) fn insert_received_component(
        &mut self,
        remote_entity: &RemoteEntity,
        component_kind: &ComponentKind,
        component: Box<dyn Replicate>,
    ) {
        self.incoming_components.insert((*remote_entity, *component_kind), component);
    }

    pub(crate) fn insert_received_update(
        &mut self,
        tick: Tick,
        global_entity: &GlobalEntity,
        component_update: ComponentUpdate
    ) {
        self.incoming_updates.push((tick, *global_entity, component_update));
    }

    pub fn on_entity_channel_opened(
        &mut self,
        in_scope_entities: &dyn InScopeEntities,
        // converter: &dyn LocalEntityAndGlobalEntityConverter,
        global_entity: &GlobalEntity
    ) {
        self.waitlist.on_entity_channel_opened(in_scope_entities, global_entity);
    }

    pub fn process_world_events<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        spawner: &mut dyn GlobalEntitySpawner<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        local_entity_map: &mut LocalEntityMap,
        component_kinds: &ComponentKinds,
        world: &mut W,
        now: &Instant,
        incoming_messages: Vec<EntityMessage<RemoteEntity>>,
    ) -> Vec<EntityEvent> {

        self.process_updates(
            global_world_manager,
            local_entity_map.entity_converter(),
            spawner.to_converter(),
            component_kinds,
            world,
            now,
        );
        self.process_incoming_messages(
            spawner,
            global_world_manager,
            local_entity_map,
            world,
            now,
            incoming_messages,
        );

        std::mem::take(&mut self.incoming_events)
    }

    /// Process incoming Entity messages.
    ///
    /// * Emits client events corresponding to any [`EntityMessage`] received
    /// Store
    pub fn process_incoming_messages<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        spawner: &mut dyn GlobalEntitySpawner<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        local_entity_map: &mut LocalEntityMap,
        world: &mut W,
        now: &Instant,
        incoming_messages: Vec<EntityMessage<RemoteEntity>>,
    ) {
        self.process_ready_messages(
            spawner,
            global_world_manager,
            local_entity_map,
            world,
            incoming_messages,
        );
        let world_converter = spawner.to_converter();
        self.process_waitlist_messages(
            local_entity_map.entity_converter(),
            world_converter,
            world,
            now,
        );
    }

    /// For each [`EntityMessage`] that can be executed now,
    /// execute it and emit a corresponding event.
    fn process_ready_messages<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        spawner: &mut dyn GlobalEntitySpawner<E>,
        global_world_manager: &dyn GlobalWorldManagerType,
        local_entity_map: &mut LocalEntityMap,
        world: &mut W,
        incoming_messages: Vec<EntityMessage<RemoteEntity>>,
    ) {
        // execute the action and emit an event
        for message in incoming_messages {
            // info!("Processing EntityMessage: {:?}", message);
            match message {
                EntityMessage::Spawn(remote_entity) => {
                    // set up entity
                    let world_entity = world.spawn_entity();
                    let global_entity = spawner.spawn(world_entity, Some(remote_entity));
                    if local_entity_map.contains_remote_entity(&remote_entity) {
                        // mapped remote entity already when reserving global entity
                    } else {
                        local_entity_map.insert_with_remote_entity(global_entity, remote_entity);
                    }

                    self.incoming_events
                        .push(EntityEvent::Spawn(global_entity));
                }
                EntityMessage::Despawn(remote_entity) => {
                    let global_entity = local_entity_map.remove_by_remote_entity(&remote_entity);
                    let world_entity = spawner.global_entity_to_entity(&global_entity).unwrap();

                    // Generate event for each component, handing references off just in
                    // case
                    if let Some(component_kinds) =
                        global_world_manager.component_kinds(&global_entity)
                    {
                        for component_kind in component_kinds {
                            self.process_remove(world, &global_entity, &world_entity, &component_kind);
                        }
                    }

                    world.despawn_entity(&world_entity);

                    self.incoming_events
                        .push(EntityEvent::Despawn(global_entity));
                }
                EntityMessage::InsertComponent(remote_entity, component_kind) => {
                    let component = self.incoming_components
                        .remove(&(remote_entity, component_kind))
                        .unwrap();

                    if local_entity_map.contains_remote_entity(&remote_entity) {
                        let global_entity = *local_entity_map.global_entity_from_remote(&remote_entity).unwrap();
                        let world_entity = spawner.global_entity_to_entity(&global_entity).unwrap();

                        let mut reserver = local_entity_map.global_entity_reserver(global_world_manager, spawner);
                        
                        self.process_insert(
                            world,
                            &mut reserver,
                            &global_entity,
                            &world_entity,
                            component,
                            &component_kind,
                        );
                    } else {
                        // entity may have despawned on disconnect or something similar?
                        warn!("received InsertComponent message for nonexistant entity");
                    }
                }
                EntityMessage::RemoveComponent(remote_entity, component_kind) => {
                    let global_entity = local_entity_map.global_entity_from_remote(&remote_entity).unwrap();
                    let world_entity = spawner.global_entity_to_entity(global_entity).unwrap();
                    self.process_remove(world, global_entity, &world_entity, &component_kind);
                }
                EntityMessage::Noop => {
                    // do nothing
                }
                msg => {
                    let msg_type = msg.get_type();
                    let event = match msg_type {
                        EntityMessageType::EnableDelegationResponse |
                        EntityMessageType::MigrateResponse |
                        EntityMessageType::RequestAuthority => {
                            let msg = msg.to_host_message();
                            msg.to_event(local_entity_map)
                        }
                        EntityMessageType::ReleaseAuthority => {
                            let EntityMessage::ReleaseAuthority(owned_entity) = msg else {
                                panic!("");
                            };
                            match owned_entity {
                                OwnedLocalEntity::Remote(remote_entity) => {
                                    let remote_entity = RemoteEntity::new(remote_entity);
                                    let global_entity = local_entity_map.global_entity_from_remote(&remote_entity).unwrap();
                                    EntityEvent::ReleaseAuthority(*global_entity)
                                }
                                OwnedLocalEntity::Host(host_entity) => {
                                    let host_entity = HostEntity::new(host_entity);
                                    let global_entity = local_entity_map.global_entity_from_host(&host_entity).unwrap();
                                    EntityEvent::ReleaseAuthority(*global_entity)
                                }
                            }
                        }
                        _ => msg.to_event(local_entity_map)
                    };
                    self.incoming_events.push(event);
                }
            }
        }
    }

    fn process_insert<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        world: &mut W,
        converter: &mut dyn InScopeEntitiesMut,
        global_entity: &GlobalEntity,
        world_entity: &E,
        component: Box<dyn Replicate>,
        component_kind: &ComponentKind,
    ) {
        if let Some(remote_entity_set) = component.relations_waiting() {

            // let name = component.name();
            // warn!(
            //     "Remote World Manager: waitlisting entity {:?}'s component {:?} for insertion. Waiting on Entities: {:?}",
            //     global_entity, &name, remote_entity_set,
            // );
            
            if let Ok(global_entity_set) =
                converter.get_or_reserve_global_entity_set_from_remote_entity_set(remote_entity_set)
            {
                // info!(
                //     "Remote World Manager: queueing component {:?} for entity {:?} in waitlist",
                //     &name, global_entity
                // );

                self.waitlist.waitlist_queue_entity(converter, &global_entity, component, component_kind, &global_entity_set);
            } else {
                panic!("Remote World Manager: cannot convert remote entity set to global entity set for waitlisting");
            }
        } else {
            self.finish_insert(
                world,
                global_entity,
                &world_entity,
                component,
                component_kind,
            );
        }
    }

    fn finish_insert<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        world: &mut W,
        global_entity: &GlobalEntity,
        world_entity: &E,
        component: Box<dyn Replicate>,
        component_kind: &ComponentKind,
    ) {
        // let name = component.name();
        // info!(
        //     "Remote World Manager: finish inserting component {:?} for entity {:?}",
        //     &name, global_entity
        // );
        
        world.insert_boxed_component(&world_entity, component);

        self.incoming_events
            .push(EntityEvent::InsertComponent(*global_entity, *component_kind));
    }

    fn process_remove<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        world: &mut W,
        global_entity: &GlobalEntity,
        world_entity: &E,
        component_kind: &ComponentKind,
    ) {
        if self.waitlist.process_remove(&global_entity, &component_kind) {
            return;
        }
        // Remove from world
        if let Some(component) = world.remove_component_of_kind(&world_entity, &component_kind) {
            // Send out event
            self.incoming_events
                .push(EntityEvent::RemoveComponent(*global_entity, component));
        }
    }

    fn process_waitlist_messages<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        local_converter: &dyn LocalEntityAndGlobalEntityConverter,
        world_converter: &dyn EntityAndGlobalEntityConverter<E>,
        world: &mut W,
        now: &Instant,
    ) {
        for (global_entity, component_kind, component) in self.waitlist.entities_to_insert(now, local_converter) {
            let world_entity = world_converter
                .global_entity_to_entity(&global_entity)
                .unwrap();
            self.finish_insert(
                world,
                &global_entity,
                &world_entity,
                component,
                &component_kind,
            );
        }
    }

    /// Process incoming Entity updates.
    ///
    /// * Emits client events corresponding to any [`EntityMessage`] received
    /// Store
    pub fn process_updates<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        in_scope_entities: &dyn InScopeEntities,
        local_converter: &dyn LocalEntityAndGlobalEntityConverter,
        world_converter: &dyn EntityAndGlobalEntityConverter<E>,
        component_kinds: &ComponentKinds,
        world: &mut W,
        now: &Instant,
    ) {
        self.process_ready_updates(
            in_scope_entities,
            local_converter,
            world_converter,
            component_kinds,
            world,
        );
        self.process_waitlist_updates(local_converter, world_converter, world, now);
    }

    /// Process component updates from raw bits for a given entity
    fn process_ready_updates<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        in_scope_entities: &dyn InScopeEntities,
        local_converter: &dyn LocalEntityAndGlobalEntityConverter,
        world_converter: &dyn EntityAndGlobalEntityConverter<E>,
        component_kinds: &ComponentKinds,
        world: &mut W,
    ) {
        let incoming_updates = std::mem::take(&mut self.incoming_updates);
        for (tick, global_entity, component_kind) in self.waitlist.process_ready_updates(
                in_scope_entities,
                local_converter,
                world_converter,
                component_kinds,
                world,
                incoming_updates
        ) {
            self.incoming_events.push(EntityEvent::UpdateComponent(
                tick,
                global_entity,
                component_kind,
            ));
        }
    }

    fn process_waitlist_updates<E: Copy + Eq + Hash + Send + Sync, W: WorldMutType<E>>(
        &mut self,
        local_converter: &dyn LocalEntityAndGlobalEntityConverter,
        world_converter: &dyn EntityAndGlobalEntityConverter<E>,
        world: &mut W,
        now: &Instant,
    ) {
        for (tick, global_entity, component_kind) in self.waitlist.process_waitlist_updates(
            local_converter,
            world_converter,
            world,
            now,
        ) {
            self.incoming_events.push(EntityEvent::UpdateComponent(
                tick,
                global_entity,
                component_kind,
            ));
        }
    }
}
