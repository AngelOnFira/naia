use std::{collections::HashMap, fmt::Debug, hash::Hash};

use crate::{messages::channels::receivers::reliable_receiver::ReliableReceiver, world::sync::{ReceiverEngine, EntityChannelReceiver}, EntityMessage, HostType, MessageIndex};

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

    /// Buffer a read [`EntityMessage`] so that it can be processed later
    pub fn buffer_message(&mut self, message_index: MessageIndex, message: EntityMessage<E>) {
        self.receiver.buffer_message(message_index, message);
    }

    /// Read all buffered [`EntityMessage`] inside the `receiver` and process them.
    ///
    /// Outputs the list of [`EntityMessage`] that can be executed now, buffer the rest
    /// into each entity's [`EntityChannelReceiver`]
    pub fn receive_messages(&mut self) -> Vec<EntityMessage<E>> {
        let incoming_messages = self.receiver.receive_messages();
        for (message_index, message) in incoming_messages {
            self.engine.accept_message(message_index, message);
        }
        self.engine.receive_messages()
    }

    pub(crate) fn get_world(&self) -> &HashMap<E, EntityChannelReceiver> {
        self.engine.get_world()
    }
}