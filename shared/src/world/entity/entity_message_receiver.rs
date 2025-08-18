use std::{fmt::Debug, hash::Hash};

use crate::{messages::channels::receivers::reliable_receiver::ReliableReceiver, world::sync::RemoteEngine, EntityMessage, MessageIndex};

pub struct EntityMessageReceiver;

impl EntityMessageReceiver {

    /// Buffer a read [`EntityMessage`] so that it can be processed later
    pub fn buffer_message<E: Copy + Hash + Eq + Debug>(
        receiver: &mut ReliableReceiver<EntityMessage<E>>,
        message_index: MessageIndex,
        message: EntityMessage<E>,
    ) {
        receiver.buffer_message(message_index, message);
    }

    /// Read all buffered [`EntityMessage`] inside the `receiver` and process them.
    ///
    /// Outputs the list of [`EntityMessage`] that can be executed now, buffer the rest
    /// into each entity's [`EntityChannelReceiver`]
    pub fn receive_messages<E: Copy + Hash + Eq + Debug>(
        receiver: &mut ReliableReceiver<EntityMessage<E>>,
        remote_engine: &mut RemoteEngine<E>
    ) -> Vec<EntityMessage<E>> {
        let incoming_messages = receiver.receive_messages();
        for (message_index, message) in incoming_messages {
            remote_engine.accept_message(message_index, message);
        }
        remote_engine.receive_messages()
    }
}