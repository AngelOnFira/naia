use std::{hash::Hash, collections::HashMap};

use crate::{
    sequence_equal_or_less_than,
    world::{
        sync::{
            auth_channel::AuthChannel,
            component_channel::ComponentChannel,
        },
        entity::ordered_ids::OrderedIds
    },
    ComponentKind, EntityMessage, EntityMessageType, MessageIndex
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityChannelState {
    Despawned,
    Spawned,
}

pub(crate) struct EntityChannel {
    component_channels: HashMap<ComponentKind, ComponentChannel>,
    outgoing_messages: Vec<EntityMessage<()>>,
    state: EntityChannelState,
    auth_channel: AuthChannel,
    buffered_messages: OrderedIds<EntityMessage<()>>,
    last_spawn_id: Option<MessageIndex>,
}

impl EntityChannel {
    pub(crate) fn new() -> Self {
        Self {
            component_channels: HashMap::new(),
            outgoing_messages: Vec::new(),
            state: EntityChannelState::Despawned,
            auth_channel: AuthChannel::new(),
            buffered_messages: OrderedIds::new(),
            last_spawn_id: None,
        }
    }

    pub(crate) fn accept_message(
        &mut self,
        id: MessageIndex,
        msg: EntityMessage<()>,
    ) {
        if let Some(last_spawn_id) = self.last_spawn_id {
            if sequence_equal_or_less_than(id, last_spawn_id) {
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

    fn process_messages(&mut self) {
        loop {
            let Some((id, msg)) = self.buffered_messages.peek_front() else {
                break;
            };
            match msg.get_type() {
                EntityMessageType::SpawnEntity => {
                    if self.state != EntityChannelState::Despawned {
                        break;
                    }

                    let id = *id;

                    self.state = EntityChannelState::Spawned;
                    self.last_spawn_id = Some(id);
                    // clear buffered messages less than or equal to the last spawn id
                    self.buffered_messages.pop_front_until_and_excluding(id);

                    self.pop_front_into_outgoing();

                    // Drain the auth channel and append the messages to the outgoing events
                    self.auth_channel.buffer_pop_front_until_and_excluding(id);
                    self.auth_channel.drain_messages_into(&mut self.outgoing_messages);

                    // Drain the component channel and append the messages to the outgoing events
                    for (component_kind, component_channel) in self.component_channels.iter_mut() {
                        component_channel.buffer_pop_front_until_and_excluding(id);
                        component_channel.drain_messages_into(component_kind, &mut self.outgoing_messages);
                    }
                },
                EntityMessageType::DespawnEntity => {
                    if self.state != EntityChannelState::Spawned {
                        break;
                    }

                    self.state = EntityChannelState::Despawned;
                    self.last_spawn_id = None;

                    self.auth_channel.reset();
                    self.component_channels.clear();

                    self.pop_front_into_outgoing();

                    // clear the buffer
                    self.buffered_messages.clear();
                },
                EntityMessageType::InsertComponent | EntityMessageType::RemoveComponent => {

                    let (id, msg) = self.buffered_messages.pop_front().unwrap();
                    
                    let component_kind = msg.component_kind().unwrap();
                    let component_channel = self.component_channels
                        .entry(component_kind)
                        .or_insert_with(ComponentChannel::new);

                    component_channel.accept_message(id, msg);
                    
                    if self.state != EntityChannelState::Spawned {
                        continue;
                    }

                    component_channel.drain_messages_into(&component_kind, &mut self.outgoing_messages);
                }
                EntityMessageType::PublishEntity | EntityMessageType::UnpublishEntity |
                EntityMessageType::EnableDelegationEntity | EntityMessageType::DisableDelegationEntity |
                EntityMessageType::RequestAuthority | EntityMessageType::ReleaseAuthority |
                EntityMessageType::UpdateAuthority | EntityMessageType::EnableDelegationEntityResponse | EntityMessageType::EntityMigrateResponse => {
                    let (id, msg) = self.buffered_messages.pop_front().unwrap();

                    self.auth_channel.accept_message(id, msg);

                    if self.state != EntityChannelState::Spawned {
                        continue;
                    }

                    self.auth_channel.drain_messages_into(&mut self.outgoing_messages);
                }
                EntityMessageType::Noop => {
                    // Drop it
                }
            }
        }
    }

    fn pop_front_into_outgoing(&mut self) {
        let (_, msg) = self.buffered_messages.pop_front().unwrap();
        self.outgoing_messages.push(msg);
    }
}