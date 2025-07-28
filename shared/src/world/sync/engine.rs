use std::{hash::Hash, collections::HashMap};

use crate::{world::{sync::{entity_channel::EntityChannel, config::EngineConfig}, entity::entity_message::EntityMessage}, MessageIndex};

pub struct Engine<E: Copy + Hash + Eq> {
    pub config: EngineConfig,
    outgoing_events: Vec<EntityMessage<E>>,
    entity_channels: HashMap<E, EntityChannel>,
}

impl<E: Copy + Hash + Eq> Default for Engine<E> {
    fn default() -> Self {
        Self {
            config: EngineConfig::default(),
            outgoing_events: Vec::new(),
            entity_channels: HashMap::new(),
        }
    }
}

impl<E: Copy + Hash + Eq> Engine<E> {

    /// Feed a de-duplicated, unordered message into the engine.
    pub fn accept_message(
        &mut self,
        id: MessageIndex,
        msg: EntityMessage<E>
    ) {
        let Some(entity) = msg.entity() else {
            // was a no-op message
            return;
        };
        // If the entity channel does not exist, create it
        let entity_channel = self.entity_channels
            .entry(entity)
            .or_insert_with(EntityChannel::new);
        
        entity_channel.accept_message(id, msg.strip_entity());

        entity_channel.drain_messages_into(entity, &mut self.outgoing_events);
    }

    /// Drain messages from the engine in appropriate order.
    pub fn receive_messages(&mut self) -> Vec<EntityMessage<E>> {
        std::mem::take(&mut self.outgoing_events)
    }
}