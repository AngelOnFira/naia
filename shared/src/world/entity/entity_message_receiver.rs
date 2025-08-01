use std::{fmt::Debug, hash::Hash};

use log::info;

use crate::{messages::channels::receivers::reliable_receiver::ReliableReceiver, world::{component::component_kinds::ComponentKind, sync::Engine}, EntityMessage, HostType, MessageIndex};

pub struct EntityMessageReceiver<E: Copy + Hash + Eq + Debug> {
    receiver: ReliableReceiver<EntityMessage<E>>,
    engine: Engine<E>,
}

impl<E: Copy + Hash + Eq + Debug> EntityMessageReceiver<E> {
    pub fn new(host_type: HostType) -> Self {
        Self {
            receiver: ReliableReceiver::new(),
            engine: Engine::new(host_type),
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
        let incoming_messages = self.receiver.receive_messages();
        for (message_index, message) in incoming_messages {
            self.engine.accept_message(message_index, message);
        }
        self.engine.receive_messages()
    }
}