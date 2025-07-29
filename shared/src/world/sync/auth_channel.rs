use crate::{world::entity::ordered_ids::OrderedIds, EntityMessage, MessageIndex};

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

    pub(crate) fn reset(&mut self) {
        self.state = EntityAuthChannelState::Unpublished;
        self.buffered_messages.clear();
        self.outgoing_messages.clear();
    }

    pub(crate) fn drain_messages_into(
        &mut self,
        outgoing_messages: &mut Vec<EntityMessage<()>>,
    ) {
        // Drain the auth channel and append the messages to the outgoing events
        outgoing_messages.append(&mut self.outgoing_messages);
    }
    
    pub(crate) fn buffer_pop_front_until_and_excluding(&mut self, id: MessageIndex) {
        self.buffered_messages.pop_front_until_and_excluding(id);
    }

    pub(crate) fn accept_message(&mut self, id: MessageIndex, msg: EntityMessage<()>) {
        self.buffered_messages.push_back(id, msg);
        self.process_messages();
    }
    
    fn process_messages(&mut self) {
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