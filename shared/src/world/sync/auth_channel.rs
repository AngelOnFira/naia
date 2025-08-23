use crate::{
    world::sync::{entity_channel_receiver::EntityChannelState, auth_channel_sender::AuthChannelSender, auth_channel_receiver::AuthChannelReceiver},
    EntityAuthStatus, EntityCommand, EntityMessage,
    EntityMessageType, HostType, MessageIndex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntityAuthChannelState {
    Unpublished,
    Published,
    Delegated,
}

pub(crate) struct AuthChannel {
    state: EntityAuthChannelState,
    sender: AuthChannelSender,
    receiver: AuthChannelReceiver,
}

impl AuthChannel {
    pub(crate) fn new(host_type: HostType) -> Self {
        let state = match host_type {
            HostType::Client => EntityAuthChannelState::Unpublished,
            HostType::Server => EntityAuthChannelState::Published,
        };
        Self {
            state,
            sender: AuthChannelSender::new(),
            receiver: AuthChannelReceiver::new(),
        }
    }

    pub(crate) fn validate_command(
        &mut self,
        command: &EntityCommand,
    ) {
        match command.get_type() {
            EntityMessageType::Publish => {
                if self.state != EntityAuthChannelState::Unpublished {
                    panic!("Cannot publish an entity that is already published");
                }
                self.state = EntityAuthChannelState::Published;
            }
            EntityMessageType::Unpublish => {
                if self.state != EntityAuthChannelState::Published {
                    panic!("Cannot unpublish an entity that is not published");
                }
                self.state = EntityAuthChannelState::Unpublished;
            }
            EntityMessageType::EnableDelegation => {
                if self.state != EntityAuthChannelState::Published {
                    panic!("Cannot enable delegation on an entity that is not published");
                }
                self.state = EntityAuthChannelState::Delegated;
            }
            EntityMessageType::DisableDelegation => {
                if self.state != EntityAuthChannelState::Delegated {
                    panic!("Cannot disable delegation on an entity that is not delegated");
                }
                self.state = EntityAuthChannelState::Published;
            }
            EntityMessageType::SetAuthority => {
                if self.state != EntityAuthChannelState::Delegated {
                    panic!("Cannot set authority on an entity that is not delegated");
                }

                let EntityCommand::SetAuthority(_, _entity, status) = command else {
                    panic!("Expected SetAuthority command");
                };

                match status {
                    EntityAuthStatus::Available => {
                        todo!()
                    }
                    EntityAuthStatus::Requested => {
                        todo!()
                    }
                    EntityAuthStatus::Granted => {
                        todo!()
                    }
                    EntityAuthStatus::Releasing => {
                        todo!()
                    }
                    EntityAuthStatus::Denied => {
                        todo!()
                    }
                }
            }
            _ => {
                panic!("Unsupported command type for AuthChannelSender");
            }
        }
    }
    
    pub(crate) fn send_command(
        &mut self,
        command: EntityCommand,
    ) {
        self.sender.send_command(command);
    }

    pub(crate) fn sender_drain_messages_into(&mut self, commands: &mut Vec<EntityCommand>) {
        self.sender.drain_messages_into(commands);
    }

    /// Is invoked by `EntityChannel` when the entity despawns; this wipes all buffered state so a future *re‑spawn* starts clean.
    pub(crate) fn receiver_reset(&mut self) {
        self.receiver.reset();
    }

    pub(crate) fn receiver_reset_next_subcommand_id(&mut self) {
        self.receiver.reset_next_subcommand_id();
    }

    pub(crate) fn receiver_drain_messages_into(
        &mut self,
        outgoing_messages: &mut Vec<EntityMessage<()>>,
    ) {
        self.receiver.drain_messages_into(outgoing_messages);
    }

    pub(crate) fn receiver_buffer_pop_front_until_and_including(&mut self, id: MessageIndex) {
        self.receiver.buffer_pop_front_until_and_including(id);
    }

    pub(crate) fn receiver_buffer_pop_front_until_and_excluding(&mut self, id: MessageIndex) {
        self.receiver.buffer_pop_front_until_and_excluding(id);
    }

    pub(crate) fn receiver_receive_message(
        &mut self,
        entity_state: EntityChannelState,
        id: MessageIndex,
        msg: EntityMessage<()>,
    ) {
        self.receiver.receive_message(entity_state, id, msg);
    }

    pub(crate) fn receiver_process_messages(&mut self, entity_state: EntityChannelState) {
        self.receiver.process_messages(entity_state);
    }
}