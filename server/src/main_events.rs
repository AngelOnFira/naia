use std::{collections::HashMap, marker::PhantomData, mem, vec::IntoIter};

use naia_shared::{Message, MessageContainer, MessageKind};

use crate::{world_events, user::UserKey, MainUser, NaiaServerError};

pub struct MainEvents {
    connections: Vec<UserKey>,
    disconnections: Vec<(UserKey, MainUser)>,
    errors: Vec<NaiaServerError>,
    auths: HashMap<MessageKind, Vec<(UserKey, MessageContainer)>>,
    empty: bool,
}

impl MainEvents {
    pub(crate) fn new() -> Self {
        Self {
            connections: Vec::new(),
            disconnections: Vec::new(),
            errors: Vec::new(),
            auths: HashMap::new(),
            empty: true,
        }
    }

    // Public

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    pub fn read<V: MainEvent>(&mut self) -> V::Iter {
        return V::iter(self);
    }

    pub fn has<V: MainEvent>(&self) -> bool {
        return V::has(self);
    }

    // This method is exposed for adapter crates ... prefer using Events.read::<SomeEvent>() instead.
    pub fn has_auths(&self) -> bool {
        !self.auths.is_empty()
    }
    pub fn take_auths(&mut self) -> HashMap<MessageKind, Vec<(UserKey, MessageContainer)>> {
        mem::take(&mut self.auths)
    }

    // Crate-public

    pub(crate) fn push_connection(&mut self, user_key: &UserKey) {
        self.connections.push(*user_key);
        self.empty = false;
    }

    pub(crate) fn push_disconnection(&mut self, user_key: &UserKey, user: MainUser) {
        self.disconnections.push((*user_key, user));
        self.empty = false;
    }

    pub(crate) fn push_auth(&mut self, user_key: &UserKey, auth_message: MessageContainer) {
        let message_type_id = auth_message.kind();
        if !self.auths.contains_key(&message_type_id) {
            self.auths.insert(message_type_id, Vec::new());
        }
        let list = self.auths.get_mut(&message_type_id).unwrap();
        list.push((*user_key, auth_message));
        self.empty = false;
    }

    pub(crate) fn push_error(&mut self, error: NaiaServerError) {
        self.errors.push(error);
        self.empty = false;
    }
}

// Event Trait
pub trait MainEvent {
    type Iter;

    fn iter(events: &mut MainEvents) -> Self::Iter;

    fn has(events: &MainEvents) -> bool;
}

// ConnectEvent
pub struct ConnectEvent;
impl MainEvent for ConnectEvent {
    type Iter = IntoIter<UserKey>;

    fn iter(events: &mut MainEvents) -> Self::Iter {
        let list = std::mem::take(&mut events.connections);
        return IntoIterator::into_iter(list);
    }

    fn has(events: &MainEvents) -> bool {
        !events.connections.is_empty()
    }
}

// DisconnectEvent
pub struct DisconnectEvent;
impl MainEvent for DisconnectEvent {
    type Iter = IntoIter<(UserKey, MainUser)>;

    fn iter(events: &mut MainEvents) -> Self::Iter {
        let list = std::mem::take(&mut events.disconnections);
        return IntoIterator::into_iter(list);
    }

    fn has(events: &MainEvents) -> bool {
        !events.disconnections.is_empty()
    }
}

// Error Event
pub struct ErrorEvent;
impl MainEvent for ErrorEvent {
    type Iter = IntoIter<NaiaServerError>;

    fn iter(events: &mut MainEvents) -> Self::Iter {
        let list = std::mem::take(&mut events.errors);
        return IntoIterator::into_iter(list);
    }

    fn has(events: &MainEvents) -> bool {
        !events.errors.is_empty()
    }
}

// Auth Event
pub struct AuthEvent<M: Message> {
    phantom_m: PhantomData<M>,
}
impl<M: Message> MainEvent for AuthEvent<M> {
    type Iter = IntoIter<(UserKey, M)>;

    fn iter(events: &mut MainEvents) -> Self::Iter {
        let message_kind: MessageKind = MessageKind::of::<M>();
        return if let Some(messages) = events.auths.remove(&message_kind) {
            IntoIterator::into_iter(world_events::read_messages(messages))
        } else {
            IntoIterator::into_iter(Vec::new())
        };
    }

    fn has(events: &MainEvents) -> bool {
        let message_kind: MessageKind = MessageKind::of::<M>();
        return events.auths.contains_key(&message_kind);
    }
}