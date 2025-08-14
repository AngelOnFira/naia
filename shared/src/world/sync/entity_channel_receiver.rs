//! ## `EntityChannel` – Per‑Entity Demultiplexer
//!
//! This module owns the **state machine and buffering logic for a *single
//! entity*** travelling across an **unordered, reliable** transport.
//!
//! ---
//! ### 1 · What problem does it solve?
//! * Messages can arrive *out of order*
//! * Certain message kinds must obey **strict causal order _within_ the
//!   entity** (e.g. a component can’t be inserted before the entity exists).
//!
//! `EntityChannel` absorbs the raw `EntityMessage<()>` stream, re‑orders and
//! filters it, and emits **ready‑to‑apply** messages in the *only* sequence
//! the game‑logic needs to respect.
//!
//! ---
//! ### 2 · State machine
//!
//! ```text
//!                 +-----------------------------+
//!                 |   Despawned (initial)       |
//!                 +-----------------------------+
//!                     | SpawnEntity(idₛ)  ▲
//!                     v                   |
//!                 +-----------------------------+
//!                 |     Spawned                 |
//!                 +-----------------------------+
//!                     | DespawnEntity(id_d)     |
//!                     +-------------------------+
//! ```
//!
//! * **`Despawned`** – entity is not present; buffers *only* the next
//!   `SpawnEntity` plus any later auth/component messages (they will flush
//!   once the spawn occurs).
//! * **`Spawned`** – entity is live; forwards component/auth messages to the
//!   corresponding sub‑channels and drains their output immediately.
//!
//! ---
//! ### 3 · Message ingest algorithm
//! 1. **Gating by `last_epoch_id `**
//!    A message whose `id ≤ last_epoch_id ` is *by definition* older than the
//!    authoritative `SpawnEntity`; drop it to guarantee *at‑most‑once
//!    semantics*; wrap‑around itself is handled automatically by the
//!    wrap‑safe `u16` comparison helpers—no epoch reset is performed.
//! 2. **Buffered queue (`OrderedIds`)**  
//!    Messages are pushed into `buffered_messages`, ordered by the `u16`
//!    sequence with wrap‑safe comparison.  
//!    `process_messages()` iterates from the head while the next candidate is
//!    *legal* under the current FSM state.
//! 3. **Draining**  
//!    Once a message is applied, it is moved into `outgoing_messages`.  
//!    `Engine::drain_messages_into` later annotates them with the concrete
//!    entity handle and forwards them to the ECS.
//!
//! ---
//! ### 4 · Sub‑channels
//! * **`AuthChannel`** – publishes, unpublishes, and delegates authority.
//! * **`ComponentChannel`** (one per `ComponentKind`) – tracks insert/remove
//!   toggles, guaranteeing idempotency via its own `last_insert_id` guard.
//!
//! `EntityChannel` coordinates these sub‑channels but *never* peers inside
//! their logic; it merely aligns their buffers with the entity’s lifecycle
//! (e.g., flush everything ≤ `idₛ` at spawn, reset on despawn).
//!
//! ---
//! ### 5 · Key invariants
//! * **Spawn barrier** – No component/auth message can overtake the spawn
//!   that legitimises it.
//! * **Monotonic visibility** – Once a message has been emitted to
//!   `outgoing_messages`, the channel guarantees it will never retract or
//!   reorder that message.
//!
//! Together, these guarantees let higher layers treat the engine as if every
//! entity had its own perfect *ordered* stream—while the network enjoys the
//! performance of a single unordered reliable channel.

use std::{hash::Hash, collections::{HashMap, HashSet}};

use crate::{sequence_less_than, world::{
    sync::{
        auth_channel_receiver::AuthChannelReceiver,
        component_channel_receiver::ComponentChannelReceiver,
    },
    entity::ordered_ids::OrderedIds
}, ComponentKind, EntityMessage, EntityMessageType, HostType, MessageIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntityChannelState {
    Despawned,
    Spawned,
}

pub struct EntityChannelReceiver {
    host_type: HostType,
    component_channels: HashMap<ComponentKind, ComponentChannelReceiver>,
    outgoing_messages: Vec<EntityMessage<()>>,
    state: EntityChannelState,
    auth_channel: AuthChannelReceiver,
    buffered_messages: OrderedIds<EntityMessage<()>>,
    last_epoch_id: Option<MessageIndex>,
}

impl EntityChannelReceiver {
    pub(crate) fn new(host_type: HostType) -> Self {
        Self {
            host_type,
            component_channels: HashMap::new(),
            outgoing_messages: Vec::new(),
            state: EntityChannelState::Despawned,
            auth_channel: AuthChannelReceiver::new(),
            buffered_messages: OrderedIds::new(),
            last_epoch_id: None,
        }
    }

    pub(crate) fn accept_message(
        &mut self,
        id: MessageIndex,
        msg: EntityMessage<()>,
    ) {
        if let Some(last_epoch_id) = self.last_epoch_id {

            if last_epoch_id == id {
                panic!("EntityChannel received a message with the same id as the last epoch id. This should not happen. Message: {:?}", msg);
            }

            if sequence_less_than(id, last_epoch_id) {
                // This message is older than the last spawn message, ignore it
                return;
            }
        }

        self.buffered_messages.push_back(id, msg);

        self.process_messages();
    }

    pub(crate) fn drain_messages_into<E: Copy + Hash + Eq>(&mut self, entity: E, outgoing_events: &mut Vec<EntityMessage<E>>) {
        // Drain the entity channel and append the messages to the outgoing events
        let mut received_messages = Vec::new();
        for rmsg in std::mem::take(&mut self.outgoing_messages) {
            received_messages.push(rmsg.with_entity(entity));
        }
        outgoing_events.append(&mut received_messages);
    }

    pub(crate) fn has_component_kind(&self, component_kind: &ComponentKind) -> bool {
        self.component_channels.contains_key(component_kind)
    }

    pub(crate) fn component_kinds_intersection(
        &self,
        other_component_kinds: &HashSet<ComponentKind>
    ) -> HashSet<ComponentKind> {
        intersection_keys(other_component_kinds, &self.component_channels)
    }

    fn process_messages(&mut self) {
        loop {
            let Some((id, msg)) = self.buffered_messages.peek_front() else {
                break;
            };
            let id = *id;

            match msg.get_type() {
                EntityMessageType::Spawn => {
                    if self.state != EntityChannelState::Despawned {
                        break;
                    }

                    self.state = EntityChannelState::Spawned;
                    self.last_epoch_id = Some(id);
                    // clear buffered messages less than or equal to the last spawn id
                    self.buffered_messages.pop_front_until_and_excluding(id);

                    self.pop_front_into_outgoing();

                    // Drain the auth channel and append the messages to the outgoing events
                    self.auth_channel.buffer_pop_front_until_and_including(id);

                    // If HostType == Client, spawned entities are published by default
                    if self.host_type == HostType::Client {
                        self.auth_channel.set_published();
                    } else {
                        self.auth_channel.set_unpublished();
                    }

                    self.auth_channel.process_messages(self.state);
                    self.auth_channel.drain_messages_into(&mut self.outgoing_messages);

                    // Pop buffered messages from the component channels until and excluding the spawn id
                    // Then process the messages in the component channels
                    // Then drain the messages into the outgoing messages
                    for (component_kind, component_channel) in self.component_channels.iter_mut() {
                        component_channel.buffer_pop_front_until_and_excluding(id);
                        component_channel.process_messages(self.state);
                        component_channel.drain_messages_into(component_kind, &mut self.outgoing_messages);
                    }
                }
                EntityMessageType::Despawn => {
                    if self.state != EntityChannelState::Spawned {
                        break;
                    }

                    self.state = EntityChannelState::Despawned;
                    self.last_epoch_id = Some(id);

                    self.auth_channel.reset();
                    self.component_channels.clear();

                    self.pop_front_into_outgoing();

                    // clear the buffer
                    self.buffered_messages.clear();
                }
                EntityMessageType::InsertComponent | EntityMessageType::RemoveComponent => {

                    let (id, msg) = self.buffered_messages.pop_front().unwrap();
                    
                    let component_kind = msg.component_kind().unwrap();
                    let component_channel = self.component_channels
                        .entry(component_kind)
                        .or_insert_with(ComponentChannelReceiver::new);

                    component_channel.accept_message(self.state, id, msg);
                    component_channel.drain_messages_into(&component_kind, &mut self.outgoing_messages);
                }
                EntityMessageType::Publish | EntityMessageType::Unpublish |
                EntityMessageType::EnableDelegation | EntityMessageType::DisableDelegation |
                EntityMessageType::SetAuthority => {
                    let (id, msg) = self.buffered_messages.pop_front().unwrap();
                    
                    // info!("EntityChannel::accept_message(id={}, msgType={:?})", id, msg.get_type());

                    self.auth_channel.accept_message(self.state, id, msg);
                    self.auth_channel.drain_messages_into(&mut self.outgoing_messages);
                }
                EntityMessageType::Noop => {
                    // Drop it
                }
                msg => {
                    panic!("EntityChannel::accept_message() received an unexpected message type: {:?}", msg);
                }
            }
        }
    }

    fn pop_front_into_outgoing(&mut self) {
        let (_, msg) = self.buffered_messages.pop_front().unwrap();
        self.outgoing_messages.push(msg);
    }
}

// This function computes the intersection of keys between a `HashSet` and a `HashMap`.
use std::hash::BuildHasher;

fn intersection_keys<K, V, SA, SB>(
    a: &HashSet<K, SA>,
    b: &HashMap<K, V, SB>,
) -> HashSet<K, SA>
where
    K: Eq + Hash + Copy,
    SA: BuildHasher + Clone,
    SB: BuildHasher,
{
    let cap = a.len().min(b.len());
    let mut out = HashSet::with_capacity_and_hasher(cap, a.hasher().clone());

    if a.len() <= b.len() {
        for &k in a {
            if b.contains_key(&k) { out.insert(k); }
        }
    } else {
        for &k in b.keys() {
            if a.contains(&k) { out.insert(k); }
        }
    }
    out
}