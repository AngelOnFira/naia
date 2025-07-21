use std::hash::Hash;

use crate::{messages::channels::receivers::reliable_receiver::ReliableReceiver, world::component::component_kinds::ComponentKind, EntityMessage, MessageIndex};

pub struct EntityMessageReceiver<E: Copy + Hash + Eq> {
    receiver: ReliableReceiver<EntityMessage<E>>,
}

impl<E: Copy + Hash + Eq> EntityMessageReceiver<E> {
    pub fn new() -> Self {
        Self {
            receiver: ReliableReceiver::new(),
        }
    }

    pub fn track_hosts_redundant_remote_entity(
        &mut self,
        entity: &E,
        component_kinds: &Vec<ComponentKind>,
    ) {
        todo!();
    }

    pub fn untrack_hosts_redundant_remote_entity(&mut self, entity: &E) {
        todo!();
    }

    /// Buffer a read [`EntityMessage`] so that it can be processed later
    pub fn buffer_message(&mut self, message_index: MessageIndex, message: EntityMessage<E>) {
        self.receiver.buffer_message(message_index, message);
    }

    /// Read all buffered [`EntityMessage`] inside the `receiver` and process them.
    ///
    /// Outputs the list of [`EntityMessage`] that can be executed now, buffer the rest
    /// into each entity's [`EntityChannel`]
    pub fn receive_messages(&mut self) -> Vec<EntityMessage<E>> {
        let mut outgoing_messages = Vec::new();
        let incoming_messages = self.receiver.receive_messages();
        for (message_index, message) in incoming_messages {
            todo!();
        }
        outgoing_messages
    }
}