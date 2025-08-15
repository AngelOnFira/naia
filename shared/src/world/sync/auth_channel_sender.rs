use crate::{EntityAuthStatus, EntityCommand, EntityMessageType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityAuthChannelState {
    Unpublished,
    Published,
    Delegated,
}

pub(crate) struct AuthChannelSender {
    state: EntityAuthChannelState,
    outgoing_commands: Vec<EntityCommand>,
}

impl AuthChannelSender {
    pub(crate) fn new() -> Self {
        Self {
            state: EntityAuthChannelState::Unpublished,
            outgoing_commands: Vec::new(),
        }
    }

    pub(crate) fn accept_message(
        &mut self,
        command: EntityCommand,
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

                let EntityCommand::SetAuthority(_, status) = command else {
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

        self.outgoing_commands.push(command);
    }

    pub(crate) fn drain_messages_into(&mut self, commands: &mut Vec<EntityCommand>) {
        commands.append(&mut self.outgoing_commands);
    }
}