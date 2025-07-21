// Tests for the new sync module
#![cfg(test)]

use crate::world::host::entity_command_sender::EntityCommandManager;
use crate::world::entity::entity_message_receiver::EntityMessageReceiver;
use crate::world::entity::global_entity::GlobalEntity;

#[test]
fn smoke_create_sender_receiver() {
    let _sender = EntityCommandManager::new();
    let _receiver: EntityMessageReceiver<GlobalEntity> = EntityMessageReceiver::new();
} 