//! Authority & Delegation Channel  
//! ==============================
//! 
//! Maintains the *authoritative‑owner* state for a single entity across an
//! unordered‑reliable transport.  `AuthChannel` is a **tiny state machine**
//! that filters, buffers, and eventually forwards only *causally‑legal*
//! authority messages to the outer `EntityChannel`.
//!
//! ## High‑level purpose
//! * Decouple global out‑of‑order arrival from the strict ordering
//!   requirements of authority negotiation.
//! * Guarantee that the ECS sees at most **one semantically valid sequence**
//!   of publish / delegate / authority‑update events, even if the network
//!   reorders packets.
//!
//! ## Accepted `EntityMessage` variants
//! | Variant                              | Meaning on receive | Requires state |
//! |--------------------------------------|--------------------|----------------|
//! | `PublishEntity`                      | Make entity visible to client | `Unpublished` |
//! | `UnpublishEntity`                    | Hide / delete entity          | `Published` |
//! | `EnableDelegationEntity`             | Allow authority hand‑offs     | `Published` |
//! | `DisableDelegationEntity`            | Revoke delegation             | `Delegated` |
//! | `EntityUpdateAuthority { … }`        | Inform who currently owns it  | `Delegated` |
//!
//! ## State machine
//! ```text
//!             +--------------------+
//!             |    Unpublished     |
//!             +---------+----------+
//!                       | PublishEntity
//!                       v
//!             +--------------------+
//!             |     Published      |
//!             +----+-----------+---+
//!                  |           |
//!  UnpublishEntity |           | EnableDelegationEntity
//!                  v           v
//!             +--------------------+
//!             |     Delegated      |
//!             +-----------+--------+
//!                         | DisableDelegationEntity
//!                         +-------------------------> back to *Published*
//! ```
//! `EntityUpdateAuthority` is a self‑loop in the **Delegated** state.
//!
//! **Invariant**: The channel never exports a message that would violate
//! the canonical state graph above; thus consumers can apply events in
//! arrival order without additional checks.

use crate::{world::{entity::ordered_ids::OrderedIds, sync::entity_channel::EntityChannelState}, EntityMessage, MessageIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityAuthChannelState {
    Unpublished,
    Published,
    Delegated,
}

pub(crate) struct AuthChannel {
    state: EntityAuthChannelState,
    buffered_messages: OrderedIds<EntityMessage<()>>,
    outgoing_messages: Vec<EntityMessage<()>>,
}

impl AuthChannel {
    pub(crate) fn new() -> Self {
        Self {
            state: EntityAuthChannelState::Unpublished,
            buffered_messages: OrderedIds::new(),
            outgoing_messages: Vec::new(),
        }
    }

    pub(crate) fn new_delegated() -> Self {
        Self {
            state: EntityAuthChannelState::Delegated,
            buffered_messages: OrderedIds::new(),
            outgoing_messages: Vec::new(),
        }
    }

    /// Is invoked by `EntityChannel` when the entity despawns; this wipes all buffered state so a future *re‑spawn* starts clean.
    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }

    pub(crate) fn set_unpublished(&mut self) {
        self.state = EntityAuthChannelState::Unpublished;
    }

    pub(crate) fn set_published(&mut self) {
        self.state = EntityAuthChannelState::Published;
    }

    pub(crate) fn drain_messages_into(
        &mut self,
        outgoing_messages: &mut Vec<EntityMessage<()>>,
    ) {
        // Drain the auth channel and append the messages to the outgoing events
        outgoing_messages.append(&mut self.outgoing_messages);
    }
    
    pub(crate) fn buffer_pop_front_until_and_including(&mut self, id: MessageIndex) {
        self.buffered_messages.pop_front_until_and_including(id);
    }

    pub(crate) fn buffer_pop_front_until_and_excluding(&mut self, id: MessageIndex) {
        self.buffered_messages.pop_front_until_and_excluding(id);
    }

    pub(crate) fn accept_message(
        &mut self,
        entity_state: EntityChannelState,
        id: MessageIndex,
        msg: EntityMessage<()>,
    ) {
        self.buffered_messages.push_back(id, msg);
        self.process_messages(entity_state);
    }
    
    pub(crate) fn process_messages(&mut self, entity_state: EntityChannelState) {
        
        if entity_state != EntityChannelState::Spawned {
            // If the entity is not spawned, we do not process any messages
            return;
        }
        
        loop {

            let Some((_, msg)) = self.buffered_messages.peek_front() else {
                break;
            };

            match msg {
                EntityMessage::PublishEntity(_) => {
                    if self.state != EntityAuthChannelState::Unpublished {
                        break;
                    }

                    self.state = EntityAuthChannelState::Published;

                    self.pop_front_into_outgoing();
                }
                EntityMessage::UnpublishEntity(_) => {
                    if self.state != EntityAuthChannelState::Published {
                        break;
                    }

                    self.state = EntityAuthChannelState::Unpublished;

                    self.pop_front_into_outgoing();
                }
                EntityMessage::EnableDelegationEntity(_) => {
                    if self.state != EntityAuthChannelState::Published {
                        break;
                    }

                    self.state = EntityAuthChannelState::Delegated;

                    self.pop_front_into_outgoing();
                }
                EntityMessage::DisableDelegationEntity(_) => {
                    if self.state != EntityAuthChannelState::Delegated {
                        break;
                    }

                    self.state = EntityAuthChannelState::Published;

                    self.pop_front_into_outgoing();
                }
                EntityMessage::EntityUpdateAuthority(_, _) => {
                    if self.state != EntityAuthChannelState::Delegated {
                        break;
                    }

                    self.pop_front_into_outgoing();
                }
                EntityMessage::EntityRequestAuthority(_, _) | EntityMessage::EntityReleaseAuthority(_) |
                EntityMessage::EnableDelegationEntityResponse(_) | EntityMessage::EntityMigrateResponse(_, _) => {
                    todo!();
                }
                _ => {
                    panic!("Unexpected message type in AuthChannel: {:?}", msg);
                }
            }
        }
    }

    fn pop_front_into_outgoing(&mut self) {
        let (_, msg) = self.buffered_messages.pop_front().unwrap();
        self.outgoing_messages.push(msg);
    }
}