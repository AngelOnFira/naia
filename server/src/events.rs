use std::{hash::Hash, collections::HashMap};

use naia_shared::{Channel, ChannelKind, ComponentKind, EntityAndGlobalEntityConverter, EntityEvent, EntityResponseEvent, GlobalResponseId, Message, MessageContainer, MessageKind, Replicate, Request, Tick};

use crate::{
    main_events::{MainEvent, MainEvents},
    user::UserKey, world_events::{WorldEvent, WorldEvents,     DelegateEntityEvent, DespawnEntityEvent,
                                  EntityAuthGrantEvent, EntityAuthResetEvent, InsertComponentEvent,
                                  MessageEvent, PublishEntityEvent, RemoveComponentEvent, RequestEvent, SpawnEntityEvent,
                                  UnpublishEntityEvent, UpdateComponentEvent},
    AuthEvent, ConnectEvent, DisconnectEvent, ErrorEvent, MainUser, NaiaServerError, TickEvent,
};

pub struct Events<E: Hash + Copy + Eq + Sync + Send> {
    main_events: MainEvents,
    world_events: WorldEvents<E>,
}

impl<E: Hash + Copy + Eq + Sync + Send> Events<E> {
    pub(crate) fn new() -> Self {
        Self {
            main_events: MainEvents::new(),
            world_events: WorldEvents::new(),
        }
    }

    // Public

    pub fn is_empty(&self) -> bool {
        self.main_events.is_empty() && self.world_events.is_empty()
    }

    pub fn read<V: Event<E>>(&mut self) -> V::Iter {
        return V::iter(self);
    }

    pub fn has<V: Event<E>>(&self) -> bool {
        return V::has(self);
    }

    // This method is exposed for adapter crates ... prefer using Events.read::<SomeEvent>() instead.
    pub fn has_messages(&self) -> bool {
        self.world_events.has_messages()
    }
    pub fn take_messages(
        &mut self,
    ) -> HashMap<ChannelKind, HashMap<MessageKind, Vec<(UserKey, MessageContainer)>>> {
        self.world_events.take_messages()
    }

    // This method is exposed for adapter crates ... prefer using Events.read::<SomeEvent>() instead.
    pub fn has_requests(&self) -> bool {
        self.world_events.has_requests()
    }
    pub fn take_requests(
        &mut self,
    ) -> HashMap<
        ChannelKind,
        HashMap<MessageKind, Vec<(UserKey, GlobalResponseId, MessageContainer)>>,
    > {
        self.world_events.take_requests()
    }

    // This method is exposed for adapter crates ... prefer using Events.read::<SomeEvent>() instead.
    pub fn has_auths(&self) -> bool {
        self.main_events.has_auths()
    }
    pub fn take_auths(&mut self) -> HashMap<MessageKind, Vec<(UserKey, MessageContainer)>> {
        self.main_events.take_auths()
    }

    // These methods are exposed for adapter crates ... prefer using Events.read::<SomeEvent>() instead.
    pub fn has_inserts(&self) -> bool {
        self.world_events.has_inserts()
    }
    pub fn take_inserts(&mut self) -> Option<HashMap<ComponentKind, Vec<(UserKey, E)>>> {
        self.world_events.take_inserts()
    }

    // These methods are exposed for adapter crates ... prefer using Events.read::<SomeEvent>() instead.
    pub fn has_updates(&self) -> bool {
        self.world_events.has_updates()
    }
    pub fn take_updates(&mut self) -> Option<HashMap<ComponentKind, Vec<(UserKey, E)>>> {
        self.world_events.take_updates()
    }

    // These method are exposed for adapter crates ... prefer using Events.read::<SomeEvent>() instead.
    pub fn has_removes(&self) -> bool {
        self.world_events.has_removes()
    }
    pub fn take_removes(
        &mut self,
    ) -> Option<HashMap<ComponentKind, Vec<(UserKey, E, Box<dyn Replicate>)>>> {
        self.world_events.take_removes()
    }

    // Crate-public

    pub(crate) fn push_connection(&mut self, user_key: &UserKey) {
        self.main_events.push_connection(user_key);
    }

    pub(crate) fn push_disconnection(&mut self, user_key: &UserKey, user: MainUser) {
        self.main_events.push_disconnection(user_key, user);
    }

    pub(crate) fn push_auth(&mut self, user_key: &UserKey, auth_message: MessageContainer) {
        self.main_events.push_auth(user_key, auth_message);
    }

    pub(crate) fn push_message(
        &mut self,
        user_key: &UserKey,
        channel_kind: &ChannelKind,
        message: MessageContainer,
    ) {
        self.world_events.push_message(user_key, channel_kind, message);
    }

    pub(crate) fn push_request(
        &mut self,
        user_key: &UserKey,
        channel_kind: &ChannelKind,
        global_response_id: GlobalResponseId,
        request: MessageContainer,
    ) {
        self.world_events.push_request(user_key, channel_kind, global_response_id, request);
    }

    pub(crate) fn push_tick(&mut self, tick: Tick) {
        self.world_events.push_tick(tick);
    }

    pub(crate) fn push_error(&mut self, error: NaiaServerError) {
        self.main_events.push_error(error);
    }

    pub(crate) fn push_spawn(&mut self, user_key: &UserKey, world_entity: &E) {
        self.world_events.push_spawn(user_key, world_entity);
    }

    pub(crate) fn push_despawn(&mut self, user_key: &UserKey, world_entity: &E) {
        self.world_events.push_despawn(user_key, world_entity);
    }

    pub(crate) fn push_publish(&mut self, user_key: &UserKey, world_entity: &E) {
        self.world_events.push_publish(user_key, world_entity);
    }

    pub(crate) fn push_unpublish(&mut self, user_key: &UserKey, world_entity: &E) {
        self.world_events.push_unpublish(user_key, world_entity);
    }

    pub(crate) fn push_delegate(&mut self, user_key: &UserKey, world_entity: &E) {
        self.world_events.push_delegate(user_key, world_entity);
    }

    pub(crate) fn push_auth_grant(&mut self, user_key: &UserKey, world_entity: &E) {
        self.world_events.push_auth_grant(user_key, world_entity);
    }

    pub(crate) fn push_auth_reset(&mut self, world_entity: &E) {
        self.world_events.push_auth_reset(world_entity);
    }

    pub(crate) fn push_insert(
        &mut self,
        user_key: &UserKey,
        world_entity: &E,
        component_kind: &ComponentKind,
    ) {
        self.world_events.push_insert(user_key, world_entity, component_kind);
    }

    pub(crate) fn push_remove(
        &mut self,
        user_key: &UserKey,
        world_entity: &E,
        component: Box<dyn Replicate>,
    ) {
        self.world_events.push_remove(user_key, world_entity, component);
    }

    pub(crate) fn push_update(
        &mut self,
        user_key: &UserKey,
        world_entity: &E,
        component_kind: &ComponentKind,
    ) {
        self.world_events.push_update(user_key, world_entity, component_kind);
    }

    pub(crate) fn receive_entity_events(
        &mut self,
        converter: &dyn EntityAndGlobalEntityConverter<E>,
        user_key: &UserKey,
        entity_events: Vec<EntityEvent>,
    ) -> Vec<EntityResponseEvent> {
        self.world_events.receive_entity_events(converter, user_key, entity_events)
    }
}

// Event Trait
pub trait Event<E: Hash + Copy + Eq + Sync + Send> {
    type Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter;

    fn has(events: &Events<E>) -> bool;
}

// Connect Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for ConnectEvent {
    type Iter = <ConnectEvent as MainEvent>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <ConnectEvent as MainEvent>::iter(&mut events.main_events)
    }

    fn has(events: &Events<E>) -> bool {
        <ConnectEvent as MainEvent>::has(&events.main_events)
    }
}

// Disconnect Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for DisconnectEvent {
    type Iter = <DisconnectEvent as MainEvent>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <DisconnectEvent as MainEvent>::iter(&mut events.main_events)
    }

    fn has(events: &Events<E>) -> bool {
        <DisconnectEvent as MainEvent>::has(&events.main_events)
    }
}

// Tick Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for TickEvent {
    type Iter = <TickEvent as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <TickEvent as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <TickEvent as WorldEvent<E>>::has(&events.world_events)
    }
}

// Error Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for ErrorEvent {
    type Iter = <ErrorEvent as MainEvent>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <ErrorEvent as MainEvent>::iter(&mut events.main_events)
    }

    fn has(events: &Events<E>) -> bool {
        <ErrorEvent as MainEvent>::has(&events.main_events)
    }
}

// Auth Event
impl<E: Hash + Copy + Eq + Sync + Send, M: Message> Event<E> for AuthEvent<M> {
    type Iter = <AuthEvent<M> as MainEvent>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <AuthEvent<M> as MainEvent>::iter(&mut events.main_events)
    }

    fn has(events: &Events<E>) -> bool {
        <AuthEvent<M> as MainEvent>::has(&events.main_events)
    }
}

// Message Event
impl<E: Hash + Copy + Eq + Sync + Send, C: Channel, M: Message> Event<E> for MessageEvent<C, M> {
    type Iter = <MessageEvent<C, M> as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <MessageEvent<C, M> as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <MessageEvent<C, M> as WorldEvent<E>>::has(&events.world_events)
    }
}

// Request Event
impl<E: Hash + Copy + Eq + Sync + Send, C: Channel, Q: Request> Event<E> for RequestEvent<C, Q> {
    type Iter = <RequestEvent<C, Q> as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <RequestEvent<C, Q> as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <RequestEvent<C, Q> as WorldEvent<E>>::has(&events.world_events)
    }
}

// Spawn Entity Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for SpawnEntityEvent {
    type Iter = <SpawnEntityEvent as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <SpawnEntityEvent as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <SpawnEntityEvent as WorldEvent<E>>::has(&events.world_events)
    }
}

// Despawn Entity Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for DespawnEntityEvent {
    type Iter = <DespawnEntityEvent as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <DespawnEntityEvent as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <DespawnEntityEvent as WorldEvent<E>>::has(&events.world_events)
    }
}

// Publish Entity Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for PublishEntityEvent {
    type Iter = <PublishEntityEvent as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <PublishEntityEvent as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <PublishEntityEvent as WorldEvent<E>>::has(&events.world_events)
    }
}

// Unpublish Entity Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for UnpublishEntityEvent {
    type Iter = <UnpublishEntityEvent as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <UnpublishEntityEvent as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <UnpublishEntityEvent as WorldEvent<E>>::has(&events.world_events)
    }
}

// Delegate Entity Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for DelegateEntityEvent {
    type Iter = <DelegateEntityEvent as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <DelegateEntityEvent as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <DelegateEntityEvent as WorldEvent<E>>::has(&events.world_events)
    }
}

// Entity Auth Grant Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for EntityAuthGrantEvent {
    type Iter = <EntityAuthGrantEvent as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <EntityAuthGrantEvent as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <EntityAuthGrantEvent as WorldEvent<E>>::has(&events.world_events)
    }
}

// Entity Auth Reset Event
impl<E: Hash + Copy + Eq + Sync + Send> Event<E> for EntityAuthResetEvent {
    type Iter = <EntityAuthResetEvent as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <EntityAuthResetEvent as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <EntityAuthResetEvent as WorldEvent<E>>::has(&events.world_events)
    }
}

// Insert Component Event
impl<E: Hash + Copy + Eq + Sync + Send, C: Replicate> Event<E> for InsertComponentEvent<C> {
    type Iter = <InsertComponentEvent<C> as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <InsertComponentEvent<C> as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <InsertComponentEvent<C> as WorldEvent<E>>::has(&events.world_events)
    }
}

// Update Component Event
impl<E: Hash + Copy + Eq + Sync + Send, C: Replicate> Event<E> for UpdateComponentEvent<C> {
    type Iter = <UpdateComponentEvent<C> as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <UpdateComponentEvent<C> as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <UpdateComponentEvent<C> as WorldEvent<E>>::has(&events.world_events)
    }
}

// Remove Component Event
impl<E: Hash + Copy + Eq + Sync + Send, C: Replicate> Event<E> for RemoveComponentEvent<C> {
    type Iter = <RemoveComponentEvent<C> as WorldEvent<E>>::Iter;

    fn iter(events: &mut Events<E>) -> Self::Iter {
        <RemoveComponentEvent<C> as WorldEvent<E>>::iter(&mut events.world_events)
    }

    fn has(events: &Events<E>) -> bool {
        <RemoveComponentEvent<C> as WorldEvent<E>>::has(&events.world_events)
    }
}