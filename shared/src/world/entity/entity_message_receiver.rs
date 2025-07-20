use std::{
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
};

use crate::{messages::channels::receivers::reliable_receiver::ReliableReceiver, sequence_less_than, world::component::component_kinds::ComponentKind, EntityMessage, MessageIndex};
use crate::world::entity::ordered_ids::OrderedIds;

// keep E here! TODO: remove
pub struct EntityMessageReceiver<E: Copy + Hash + Eq> {
    receiver: ReliableReceiver<EntityMessage<E>>,
    entity_channels: HashMap<E, EntityChannel<E>>,
}

impl<E: Copy + Hash + Eq> EntityMessageReceiver<E> {
    pub fn new() -> Self {
        Self {
            receiver: ReliableReceiver::new(),
            entity_channels: HashMap::default(),
        }
    }

    pub fn track_hosts_redundant_remote_entity(
        &mut self,
        entity: &E,
        component_kinds: &Vec<ComponentKind>,
    ) {
        let mut entity_channel = EntityChannel::new(*entity);
        entity_channel.spawned = true;
        for component_kind in component_kinds {
            entity_channel
                .components
                .insert(*component_kind, ComponentChannel::new(None));
        }
        self.entity_channels.insert(*entity, entity_channel);
    }

    pub fn untrack_hosts_redundant_remote_entity(&mut self, entity: &E) {
        self.entity_channels.remove(entity);
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
            if let Some(entity) = message.entity() {
                self.entity_channels
                    .entry(entity)
                    .or_insert_with(|| EntityChannel::new(entity));
                let entity_channel = self.entity_channels.get_mut(&entity).unwrap();
                entity_channel.receive_message(message_index, message, &mut outgoing_messages);
            }
        }

        // TODO: VERY IMPORTANT! You need to figure out how to remove EntityChannels after they've been despawned!
        // keep in mind that you need to keep around entity channels to be able to receive messages for them still
        // RIGHT NOW THIS IS LEAKING MEMORY!
        // a TTL for these Entity Channels after they've been despawned is probably the way to go

        outgoing_messages
    }
}

// Entity Channel

// keep E here! TODO: remove
struct EntityChannel<E: Copy + Hash + Eq> {
    entity: E,
    last_canonical_index: Option<MessageIndex>,
    spawned: bool,
    components: HashMap<ComponentKind, ComponentChannel<E>>,
    waiting_spawns: OrderedIds<Vec<ComponentKind>>,
    waiting_despawns: OrderedIds<()>,
}

impl<E: Copy + Hash + Eq> EntityChannel<E> {
    pub fn new(entity: E) -> Self {
        Self {
            entity,
            spawned: false,
            components: HashMap::new(),
            waiting_spawns: OrderedIds::new(),
            waiting_despawns: OrderedIds::new(),
            last_canonical_index: None,
        }
    }

    /// Process the provided [`EntityMessage`]:
    ///
    /// * Checks that [`EntityMessage`] can be executed now
    /// * If so, add it to `outgoing_messages`
    /// * Else, add it to internal "waiting" buffers so we can check when the [`EntityMessage`]
    ///   can be executed
    ///
    /// ([`EntityMessage`]s might not be executable now, for example is an InsertComponent
    ///  is processed before the corresponding entity has been spawned)
    pub fn receive_message(
        &mut self,
        incoming_message_index: MessageIndex,
        incoming_message: EntityMessage<E>,
        outgoing_messages: &mut Vec<EntityMessage<E>>,
    ) {
        match incoming_message {
            EntityMessage::SpawnEntity(_, components) => {
                self.receive_spawn_entity_message(
                    incoming_message_index,
                    components,
                    outgoing_messages,
                );
            }
            EntityMessage::DespawnEntity(_) => {
                self.receive_despawn_entity_message(incoming_message_index, outgoing_messages);
            }
            EntityMessage::InsertComponent(_, component) => {
                self.receive_insert_component_message(
                    incoming_message_index,
                    component,
                    outgoing_messages,
                );
            }
            EntityMessage::RemoveComponent(_, component) => {
                self.receive_remove_component_message(
                    incoming_message_index,
                    component,
                    outgoing_messages,
                );
            }
            EntityMessage::Noop => {}
            _ => {}
        }
    }

    /// Process the entity message.
    /// When the entity is actually spawned on the client, send back an ack event
    /// to the server.
    pub fn receive_spawn_entity_message(
        &mut self,
        message_index: MessageIndex,
        components: Vec<ComponentKind>,
        outgoing_messages: &mut Vec<EntityMessage<E>>,
    ) {
        // this is the problem:
        // the point of the receiver is to de-dup a given event, like a Spawn Message here
        // we only only convert the NEWEST spawn packet into a SpawnMessage
        // so the problem we're running into is that: Two Spawn Packets are sent, 1 with components A, B, and 1 with components A, B, C
        // message_index will be the same for both, however ...

        // do not process any spawn OLDER than last received spawn index / despawn index
        if let Some(last_index) = self.last_canonical_index {
            if sequence_less_than(message_index, last_index) {
                return;
            }
        }

        if !self.spawned {
            self.spawned = true;
            outgoing_messages.push(EntityMessage::SpawnEntity(self.entity, components));

            // pop ALL waiting spawns, despawns, inserts, and removes OLDER than spawn_index
            self.receive_canonical(message_index);

            // process any waiting despawns
            if let Some((despawn_index, _)) = self.waiting_despawns.pop_front() {
                self.receive_despawn_entity_message(despawn_index, outgoing_messages);
            } else {
                // process any waiting inserts
                let mut inserted_components = Vec::new();
                for (component, component_state) in &mut self.components {
                    if let Some(insert_index) = component_state.waiting_inserts.pop_front() {
                        inserted_components.push((insert_index, *component));
                    }
                }

                for ((index, _), component) in inserted_components {
                    self.receive_insert_component_message(index, component, outgoing_messages);
                }
            }
        } else {
            // buffer spawn for later
            self.waiting_spawns.push_back(message_index, components);
        }
    }

    /// Process the entity despawn message
    /// When the entity has actually been despawned on the client, add an ack to the
    /// `outgoing_messages`
    pub fn receive_despawn_entity_message(
        &mut self,
        index: MessageIndex,
        outgoing_messages: &mut Vec<EntityMessage<E>>,
    ) {
        // do not process any despawn OLDER than last received spawn index / despawn index
        if let Some(last_index) = self.last_canonical_index {
            if sequence_less_than(index, last_index) {
                return;
            }
        }

        if self.spawned {
            self.spawned = false;
            outgoing_messages.push(EntityMessage::DespawnEntity(self.entity));

            // pop ALL waiting spawns, despawns, inserts, and removes OLDER than despawn_index
            self.receive_canonical(index);

            // set all component channels to 'inserted = false'
            for value in self.components.values_mut() {
                value.inserted = false;
            }

            // process any waiting spawns
            if let Some((spawn_index, components)) = self.waiting_spawns.pop_front() {
                self.receive_spawn_entity_message(spawn_index, components, outgoing_messages);
            }
        } else {
            // buffer despawn for later
            self.waiting_despawns.push_back(index, ());
        }
    }

    pub fn receive_insert_component_message(
        &mut self,
        index: MessageIndex,
        component: ComponentKind,
        outgoing_messages: &mut Vec<EntityMessage<E>>,
    ) {
        // do not process any insert OLDER than last received spawn index / despawn index
        if let Some(last_index) = self.last_canonical_index {
            if sequence_less_than(index, last_index) {
                return;
            }
        }

        if let std::collections::hash_map::Entry::Vacant(e) = self.components.entry(component) {
            e.insert(ComponentChannel::new(self.last_canonical_index));
        }
        let component_state = self.components.get_mut(&component).unwrap();

        // do not process any insert OLDER than last received insert / remove index for
        // this component
        if let Some(last_index) = component_state.last_canonical_index {
            if sequence_less_than(index, last_index) {
                return;
            }
        }

        if !self.spawned {
            component_state.waiting_inserts.push_back(index, ());
            return;
        }

        if !component_state.inserted {
            component_state.inserted = true;
            outgoing_messages.push(EntityMessage::InsertComponent(self.entity, component));

            // pop ALL waiting inserts, and removes OLDER than insert_index (in reference to
            // component)
            component_state.receive_canonical(index);

            // process any waiting removes
            if let Some((remove_index, _)) = component_state.waiting_removes.pop_front() {
                self.receive_remove_component_message(remove_index, component, outgoing_messages);
            }
        } else {
            // buffer insert
            component_state.waiting_inserts.push_back(index, ());
        }
    }

    pub fn receive_remove_component_message(
        &mut self,
        index: MessageIndex,
        component: ComponentKind,
        outgoing_messages: &mut Vec<EntityMessage<E>>,
    ) {
        // do not process any remove OLDER than last received spawn index / despawn index
        if let Some(last_index) = self.last_canonical_index {
            if sequence_less_than(index, last_index) {
                return;
            }
        }

        if let std::collections::hash_map::Entry::Vacant(e) = self.components.entry(component) {
            e.insert(ComponentChannel::new(self.last_canonical_index));
        }
        let component_state = self.components.get_mut(&component).unwrap();

        // do not process any remove OLDER than last received insert / remove index for
        // this component
        if let Some(last_index) = component_state.last_canonical_index {
            if sequence_less_than(index, last_index) {
                return;
            }
        }

        if component_state.inserted {
            component_state.inserted = false;
            outgoing_messages.push(EntityMessage::RemoveComponent(self.entity, component));

            // pop ALL waiting inserts, and removes OLDER than remove_index (in reference to
            // component)
            component_state.receive_canonical(index);

            // process any waiting inserts
            if let Some((insert_index, _)) = component_state.waiting_inserts.pop_front() {
                self.receive_insert_component_message(insert_index, component, outgoing_messages);
            }
        } else {
            // buffer remove
            component_state.waiting_removes.push_back(index, ());
        }
    }

    pub fn receive_canonical(&mut self, index: MessageIndex) {
        // pop ALL waiting spawns, despawns, inserts, and removes OLDER than index
        self.waiting_spawns.pop_front_until_and_including(index);
        self.waiting_despawns.pop_front_until_and_including(index);
        for component_state in self.components.values_mut() {
            component_state.receive_canonical(index);
        }

        self.last_canonical_index = Some(index);
    }
}

// Component Channel
// most of this should be public, no methods here

// keep E here! TODO: remove
pub struct ComponentChannel<E: Copy + Hash + Eq> {
    pub inserted: bool,
    pub last_canonical_index: Option<MessageIndex>,
    pub waiting_inserts: OrderedIds<()>,
    pub waiting_removes: OrderedIds<()>,

    phantom_e: PhantomData<E>,
}

impl<E: Copy + Hash + Eq> ComponentChannel<E> {
    pub fn new(canonical_index: Option<MessageIndex>) -> Self {
        Self {
            inserted: false,
            waiting_inserts: OrderedIds::new(),
            waiting_removes: OrderedIds::new(),
            last_canonical_index: canonical_index,

            phantom_e: PhantomData,
        }
    }

    pub fn receive_canonical(&mut self, index: MessageIndex) {
        // pop ALL waiting inserts, and removes OLDER than index
        self.waiting_inserts.pop_front_until_and_including(index);
        self.waiting_removes.pop_front_until_and_including(index);

        self.last_canonical_index = Some(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::any::TypeId;

    #[test]
    fn spawn_then_insert_emitted_in_order() {
        let mut receiver: EntityMessageReceiver<u8> = EntityMessageReceiver::new();

        let entity_id: u8 = 1;
        let comp = ComponentKind::from(TypeId::of::<u32>());

        // spawn first
        receiver.buffer_message(1, EntityMessage::SpawnEntity(entity_id, Vec::new()));
        // insert next
        receiver.buffer_message(2, EntityMessage::InsertComponent(entity_id, comp));

        let messages = receiver.receive_messages();

        assert_eq!(messages.len(), 2);
        matches!(messages[0], EntityMessage::SpawnEntity(e, _) if e == entity_id);
        matches!(messages[1], EntityMessage::InsertComponent(e, k) if e == entity_id && k == comp);
    }

    #[test]
    fn insert_before_spawn_reorders_correctly() {
        let mut receiver: EntityMessageReceiver<u8> = EntityMessageReceiver::new();

        let entity_id: u8 = 2;
        let comp = ComponentKind::from(TypeId::of::<u64>());

        // buffer Insert (idx 2) before Spawn (idx 1)
        receiver.buffer_message(2, EntityMessage::InsertComponent(entity_id, comp));

        // Now buffer Spawn (idx 1) after Insert already queued
        receiver.buffer_message(1, EntityMessage::SpawnEntity(entity_id, Vec::new()));

        let messages = receiver.receive_messages();

        assert_eq!(messages.len(), 2);
        matches!(messages[0], EntityMessage::SpawnEntity(e, _) if e == entity_id);
        matches!(messages[1], EntityMessage::InsertComponent(e, k) if e == entity_id && k == comp);
    }

    #[test]
    fn remove_blocked_until_insert() {
        let mut receiver: EntityMessageReceiver<u8> = EntityMessageReceiver::new();

        let entity_id: u8 = 3;
        let comp = ComponentKind::from(TypeId::of::<u16>());

        // Spawn arrives first (idx 1)
        receiver.buffer_message(1, EntityMessage::SpawnEntity(entity_id, Vec::new()));

        // Remove arrives before Insert (idx 3)
        receiver.buffer_message(3, EntityMessage::RemoveComponent(entity_id, comp));

        // Now Insert arrives later (idx 2)
        receiver.buffer_message(2, EntityMessage::InsertComponent(entity_id, comp));

        let messages = receiver.receive_messages();

        assert_eq!(messages.len(), 3);
        matches!(messages[0], EntityMessage::SpawnEntity(e, _) if e == entity_id);
        matches!(messages[1], EntityMessage::InsertComponent(e, k) if e == entity_id && k == comp);
        matches!(messages[2], EntityMessage::RemoveComponent(e, k) if e == entity_id && k == comp);
    }

    #[test]
    fn despawn_blocked_until_spawn() {
        let mut receiver: EntityMessageReceiver<u8> = EntityMessageReceiver::new();

        let entity_id: u8 = 4;

        // Despawn arrives first (idx 2), before Spawn
        receiver.buffer_message(2, EntityMessage::DespawnEntity(entity_id));

        // No message should be emitted yet
        assert!(receiver.receive_messages().is_empty());

        // Spawn arrives later (idx 1)
        receiver.buffer_message(1, EntityMessage::SpawnEntity(entity_id, Vec::new()));

        let messages = receiver.receive_messages();

        assert_eq!(messages.len(), 2);
        matches!(messages[0], EntityMessage::SpawnEntity(e, _) if e == entity_id);
        matches!(messages[1], EntityMessage::DespawnEntity(e) if e == entity_id);
    }

    #[test]
    fn per_entity_independence() {
        let mut receiver: EntityMessageReceiver<u8> = EntityMessageReceiver::new();

        let entity_a: u8 = 5;
        let entity_b: u8 = 6;

        let comp = ComponentKind::from(TypeId::of::<u32>());

        // 1) Insert for entity A (idx 2) arrives before its Spawn
        receiver.buffer_message(2, EntityMessage::InsertComponent(entity_a, comp));

        // 2) Spawn & Insert for entity B (idx 3 & 4)
        receiver.buffer_message(3, EntityMessage::SpawnEntity(entity_b, Vec::new()));
        receiver.buffer_message(4, EntityMessage::InsertComponent(entity_b, comp));

        // At this point we expect only messages for entity B, none for A yet
        let first_messages = receiver.receive_messages();
        assert!(first_messages.len() == 2);
        assert!(first_messages[0] == EntityMessage::SpawnEntity(entity_b, Vec::new()));
        assert!(first_messages[1] == EntityMessage::InsertComponent(entity_b, comp));

        // 3) Spawn for entity A (idx 1)
        receiver.buffer_message(1, EntityMessage::SpawnEntity(entity_a, Vec::new()));

        // We expect Spawn A & Insert A now
        let second_messages = receiver.receive_messages();
        assert!(second_messages.len() == 2);
        assert!(second_messages[0] == EntityMessage::SpawnEntity(entity_a, Vec::new()));
        assert!(second_messages[1] == EntityMessage::InsertComponent(entity_a, comp));
    }

    #[test]
    fn re_spawn_allowed_after_despawn() {
        let mut receiver: EntityMessageReceiver<u8> = EntityMessageReceiver::new();

        let entity_id: u8 = 7;

        receiver.buffer_message(3, EntityMessage::SpawnEntity(entity_id, Vec::new()));
        receiver.buffer_message(2, EntityMessage::DespawnEntity(entity_id));
        receiver.buffer_message(1, EntityMessage::SpawnEntity(entity_id, Vec::new()));
        
        let messages = receiver.receive_messages();

        assert_eq!(messages.len(), 3);
        matches!(messages[0], EntityMessage::SpawnEntity(e, _) if e == entity_id);
        matches!(messages[1], EntityMessage::DespawnEntity(e) if e == entity_id);
        matches!(messages[2], EntityMessage::SpawnEntity(e, _) if e == entity_id);
    }

    #[test]
    fn re_insert_after_remove() {
        let mut receiver: EntityMessageReceiver<u8> = EntityMessageReceiver::new();

        let entity_id: u8 = 8;
        let comp = ComponentKind::from(TypeId::of::<u32>());

        receiver.buffer_message(4, EntityMessage::InsertComponent(entity_id, comp));
        receiver.buffer_message(2, EntityMessage::InsertComponent(entity_id, comp));
        receiver.buffer_message(3, EntityMessage::RemoveComponent(entity_id, comp));
        receiver.buffer_message(1, EntityMessage::SpawnEntity(entity_id, Vec::new()));

        let messages = receiver.receive_messages();

        assert_eq!(messages.len(), 4);
        matches!(messages[0], EntityMessage::SpawnEntity(e, _) if e == entity_id);
        matches!(messages[1], EntityMessage::InsertComponent(e, k) if e == entity_id && k == comp);
        matches!(messages[2], EntityMessage::RemoveComponent(e, k) if e == entity_id && k == comp);
        matches!(messages[3], EntityMessage::InsertComponent(e, k) if e == entity_id && k == comp);
    }
}
