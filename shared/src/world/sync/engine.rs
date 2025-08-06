//! # `engine.rs` — Top‑Level Orchestrator
//!
//! The **`Engine<E>`** is the *single entry/exit point* between the raw,
//! unordered stream of `EntityMessage<E>` packets on the wire and the
//! **ordered, per‑entity event queue** your game logic consumes.
//! It owns *one* [`EntityChannel`] per live entity and two lightweight
//! collections for runtime bookkeeping:
//!
//! | Field | Purpose |
//! |-------|---------|
//! | `config`            | Compile‑time knobs from [`EngineConfig`] that bound the sliding window and guard‑band for wrap‑around safety. |
//! | `outgoing_events`   | Scratch buffer filled during `accept_message`; drained atomically via [`receive_messages`]. |
//! | `entity_channels`   | `HashMap<E, EntityChannel>` lazily populated on first sight of an entity. |
//!
//! ## Responsibilities
//! 1. **Channel dispatch** – routes each message to its entity’s channel,
//!    creating channels on demand.
//! 2. **Local ordering** – relies on per‑channel state machines to decide
//!    *when* a message is safe to surface; glues their outputs into a
//!    single, ready‑to‑apply Vec.
//! 3. **Zero HoLB guarantee** – because messages for unrelated entities
//!    never share the same queue, one delayed entity cannot stall others.
//!
//! ## API contracts
//!
//! ## Interaction with `EngineConfig`
//! The `Engine` never mutates sequence numbers, but it does rely on the
//! sender/receiver honouring `max_in_flight` and `flush_threshold` to
//! avoid ambiguous wrapping (`u16` rolls over every 65536).
//! *If you change these constants, do so symmetrically on both ends.*

use std::{fmt::Debug, hash::Hash, collections::HashMap};

use log::info;

use crate::{world::{sync::{entity_channel::EntityChannel, config::EngineConfig}, entity::entity_message::EntityMessage}, ComponentKind, EntityMessageType, HostType, MessageIndex};

pub struct Engine<E: Copy + Hash + Eq + Debug> {
    host_type: HostType,
    pub config: EngineConfig,
    outgoing_events: Vec<EntityMessage<E>>,
    entity_channels: HashMap<E, EntityChannel>,
}

impl<E: Copy + Hash + Eq + Debug> Engine<E> {

    pub(crate) fn new(host_type: HostType) -> Self {
        Self {
            host_type,
            config: EngineConfig::default(),
            outgoing_events: Vec::new(),
            entity_channels: HashMap::new(),
        }
    }

    /// * Idempotent*: the caller must already have deduplicated on
    /// `(MessageIndex, Entity)`; re‑injecting the same `(id, msg)` WILL panic!
    ///
    /// *Non‑blocking*: may push zero or more *ordered* events into the
    /// engine’s outgoing buffer, but never touches the ECS directly.
    pub fn accept_message(
        &mut self,
        id: MessageIndex,
        msg: EntityMessage<E>,
        log: bool,
    ) {
        match msg.get_type() {
            // If the message are responses, immediately return
            EntityMessageType::EnableDelegationEntityResponse | 
            EntityMessageType::RequestAuthority | 
            EntityMessageType::ReleaseAuthority | 
            EntityMessageType::EntityMigrateResponse => {
                self.outgoing_events.push(msg);
                todo!(); // we should handle these in a different engine
                return;
            }
            EntityMessageType::Noop => {
                return;
            }
            _ => {}
        }

        let entity = msg.entity().unwrap();

        // If the entity channel does not exist, create it
        let entity_channel = self.entity_channels
            .entry(entity)
            .or_insert_with(|| { EntityChannel::new(self.host_type) });

        if log {
            info!("Engine::accept_message(id={}, entity={:?}, msgType={:?})", id, entity, msg.get_type());
        }

        entity_channel.accept_message(id, msg.strip_entity());

        entity_channel.drain_messages_into(entity, &mut self.outgoing_events);
    }

    /// Atomically swaps out `outgoing_events`, giving the caller a Vec that
    /// *is already topologically ordered across entities*; apply each event
    /// in sequence and discard.
    pub fn receive_messages(&mut self) -> Vec<EntityMessage<E>> {
        std::mem::take(&mut self.outgoing_events)
    }

    pub fn host_has_remote_entity(&self, entity: &E) -> bool {
        self.entity_channels.contains_key(entity)
    }
    
    pub fn track_hosts_redundant_remote_entity(
        &mut self,
        entity: &E,
        component_kinds: &Vec<ComponentKind>,
    ) {
        if self.entity_channels.contains_key(entity) {
            panic!("Attempted to track hosts for redundant remote entity which already exists: {:?}", entity);
        }

        self.entity_channels.insert(*entity, EntityChannel::new_delegated(self.host_type));

        let entity_channel = self.entity_channels.get_mut(entity).unwrap();
        entity_channel.setup_delegated_components(component_kinds);
    }

    pub fn untrack_hosts_redundant_remote_entity(&mut self, entity: &E) {
        if !self.entity_channels.contains_key(entity) {
            panic!("Attempted to untrack hosts for redundant remote entity which does not exist: {:?}", entity);
        }

        self.entity_channels.remove(entity);
    }
}