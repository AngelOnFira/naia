use std::{fmt::Debug, hash::Hash};
use std::collections::HashMap;

use crate::{messages::channels::receivers::reliable_receiver::ReliableReceiver, world::{component::component_kinds::ComponentKind, sync::ReceiverEngine}, EntityMessage, HostType, MessageIndex};
use crate::world::sync::EntityChannelReceiver;

pub struct EntityMessageReceiver<E: Copy + Hash + Eq + Debug> {
    receiver: ReliableReceiver<EntityMessage<E>>,
    engine: ReceiverEngine<E>,
}

impl<E: Copy + Hash + Eq + Debug> EntityMessageReceiver<E> {
    pub fn new(host_type: HostType) -> Self {
        Self {
            receiver: ReliableReceiver::new(),
            engine: ReceiverEngine::new(host_type),
        }
    }

    pub fn host_has_redundant_remote_entity(
        &self,
        entity: &E,
    ) -> bool {
        self.engine.host_has_remote_entity(entity)
    }

    pub fn track_hosts_redundant_remote_entity(
        &mut self,
        entity: &E,
        component_kinds: &Vec<ComponentKind>,
    ) {
        self.engine.track_hosts_redundant_remote_entity(entity, component_kinds);
    }

    pub fn untrack_hosts_redundant_remote_entity(&mut self, entity: &E) {
        self.engine.untrack_hosts_redundant_remote_entity(entity);
    }

    /// Buffer a read [`EntityMessage`] so that it can be processed later
    pub fn buffer_message(&mut self, message_index: MessageIndex, message: EntityMessage<E>) {
        self.receiver.buffer_message(message_index, message);
    }

    /// Read all buffered [`EntityMessage`] inside the `receiver` and process them.
    ///
    /// Outputs the list of [`EntityMessage`] that can be executed now, buffer the rest
    /// into each entity's [`EntityChannelReceiver`]
    pub fn receive_messages(&mut self, log: bool) -> Vec<EntityMessage<E>> {
        let incoming_messages = self.receiver.receive_messages();
        for (message_index, message) in incoming_messages {
            self.engine.accept_message(message_index, message);
        }
        self.engine.receive_messages()
    }

    pub(crate) fn get_remote_world(&self) -> &HashMap<E, EntityChannelReceiver> {
        self.engine.get_remote_world()
    }
}