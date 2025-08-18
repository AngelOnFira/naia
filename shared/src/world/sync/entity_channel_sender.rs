use std::collections::HashSet;

use crate::{world::sync::auth_channel::AuthChannel, ComponentKind, EntityCommand, EntityMessageType, HostType};

pub struct EntityChannelSender {
    outgoing_commands: Vec<EntityCommand>,
    component_channels: HashSet<ComponentKind>,
    auth_channel: AuthChannel,
}

impl EntityChannelSender {
    pub(crate) fn new(host_type: HostType) -> Self {
        Self {
            outgoing_commands: Vec::new(),
            component_channels: HashSet::new(),
            auth_channel: AuthChannel::new(host_type),
        }
    }

    pub(crate) fn component_kinds(&self) -> &HashSet<ComponentKind> {
        &self.component_channels
    }

    pub(crate) fn accept_message(
        &mut self,
        command: EntityCommand,
    ) {
        match command.get_type() {
            EntityMessageType::Spawn |
            EntityMessageType::Despawn |
            EntityMessageType::RequestAuthority |
            EntityMessageType::ReleaseAuthority |
            EntityMessageType::EnableDelegationResponse |
            EntityMessageType::MigrateResponse |
            EntityMessageType::Noop => {
                panic!("These should be handled by the Engine, not the EntityChannelSender");
            }
            EntityMessageType::InsertComponent => {
                let component_kind = command.component_kind().unwrap();
                if self.component_channels.contains(&component_kind) {
                    panic!("Cannot insert a component that already exists in the entity channel");
                }
                self.component_channels.insert(component_kind);
                self.outgoing_commands.push(command);
                return;
            }
            EntityMessageType::RemoveComponent => {
                let component_kind = command.component_kind().unwrap();
                if !self.component_channels.contains(&component_kind) {
                    panic!("Cannot remove a component that does not exist in the entity channel");
                }
                self.component_channels.remove(&component_kind);
                self.outgoing_commands.push(command);
                return;
            }
            EntityMessageType::Publish | EntityMessageType::Unpublish |
            EntityMessageType::EnableDelegation | EntityMessageType::DisableDelegation |
            EntityMessageType::SetAuthority => {
                self.auth_channel.accept_command(&command);
                self.auth_channel.send_command(command);
                self.auth_channel.sender_drain_messages_into(&mut self.outgoing_commands);
                return;
            }
        }
    }

    pub(crate) fn drain_messages_into(
        &mut self,
        outgoing_commands: &mut Vec<EntityCommand>,
    ) {
        outgoing_commands.append(&mut self.outgoing_commands);
    }
}