use std::collections::{HashMap, VecDeque};

use naia_socket_shared::Instant;

use crate::{
    world::{host::host_world_manager::CommandId, sync::{EntityChannelSender, SenderEngine}},
    ChannelSender, EntityCommand, GlobalEntity, HostType,
    ReliableSender,
};

pub struct EntityMessageSender {
    sender: ReliableSender<EntityCommand>,
    engine: SenderEngine,
}

impl EntityMessageSender {
    pub fn new(host_type: HostType, resend_command_rtt_factor: f32) -> Self {
        Self {
            sender: ReliableSender::new(resend_command_rtt_factor),
            engine: SenderEngine::new(host_type),
        }
    }

    /// Unified entry point for sending commands - mirrors EntityMessageReceiver.buffer_message()
    /// Validates command through SenderEngine before forwarding to ReliableSender
    pub fn send_command(&mut self, command: EntityCommand) {
        self.engine.accept_command(command);
    }

    /// Process and collect outgoing commands - mirrors EntityMessageReceiver.receive_messages()
    pub fn collect_outgoing_commands(
        &mut self,
        now: &Instant,
        rtt_millis: &f32,
    ) -> VecDeque<(CommandId, EntityCommand)> {
        for outgoing_command in self.engine.send_commands() {
            self.sender.send_message(outgoing_command);
        }
        self.sender.collect_messages(now, rtt_millis);
        self.sender.take_next_messages()
    }

    pub fn deliver_message(
        &mut self,
        command_id: &CommandId,
    ) -> Option<EntityCommand> {
        self.sender.deliver_message(command_id)
    }

    pub fn remote_despawn_entity(&mut self, global_entity: &GlobalEntity) {
        todo!("close entity channel");
    }

    pub(crate) fn get_world(&self) -> &HashMap<GlobalEntity, EntityChannelSender> {
        self.engine.get_world()
    }
}