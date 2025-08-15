use std::collections::HashMap;

use crate::{world::sync::{EntityChannelSender, config::EngineConfig}, GlobalEntity, HostType, EntityCommand, EntityMessageType};

pub struct SenderEngine {
    host_type: HostType,
    pub config: EngineConfig,
    entity_channels: HashMap<GlobalEntity, EntityChannelSender>,
    outgoing_commands: Vec<EntityCommand>,
}

impl SenderEngine {

    pub(crate) fn new(host_type: HostType) -> Self {
        Self {
            host_type,
            config: EngineConfig::default(),
            outgoing_commands: Vec::new(),
            entity_channels: HashMap::new(),
        }
    }

    pub(crate) fn take_outgoing_commands(&mut self) -> Vec<EntityCommand> {
        std::mem::take(&mut self.outgoing_commands)
    }

    pub(crate) fn get_world(&self) -> &HashMap<GlobalEntity, EntityChannelSender> {
        &self.entity_channels
    }

    /// Main entry point - validates command and returns it if valid
    /// This mirrors ReceiverEngine.accept_message() but for outgoing commands
    pub(crate) fn accept_command(&mut self, command: EntityCommand) {

        let entity = command.entity();
        
        match command.get_type() {
            EntityMessageType::Spawn => {
                if self.entity_channels.contains_key(&entity) {
                    panic!("Cannot spawn an entity that already exists in the engine");
                }
                // If the entity channel does not exist, create it
                self.entity_channels
                    .insert(entity, EntityChannelSender::new(self.host_type));
                
                self.outgoing_commands.push(command);
                return;
            }
            EntityMessageType::Despawn => {
                if !self.entity_channels.contains_key(&entity) {
                    panic!("Cannot despawn an entity that does not exist in the engine");
                }
                // Remove the entity channel
                self.entity_channels.remove(&entity).unwrap();
                self.outgoing_commands.push(command);
                return;
            }
            // If the message are responses, immediately return
            EntityMessageType::RequestAuthority |
            EntityMessageType::ReleaseAuthority |
            EntityMessageType::EnableDelegationResponse |
            EntityMessageType::MigrateResponse => {
                self.outgoing_commands.push(command);
                todo!(); // we should handle these in a different engine
                return;
            }
            EntityMessageType::Noop => {
                return;
            }
            _ => {}
        }
        
        let Some(entity_channel) = self.entity_channels.get_mut(&entity) else {
            panic!("Cannot accept command for an entity that does not exist in the engine");
        };

        // if log {
        //     info!("Engine::accept_command(entity={:?}, msgType={:?})", entity, msg.get_type());
        // }

        entity_channel.accept_message(command);

        entity_channel.drain_messages_into(&mut self.outgoing_commands);
    }
}