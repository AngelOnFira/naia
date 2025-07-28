use std::{hash::Hash, collections::HashMap};

use crate::{world::{sync::component_channel::ComponentChannel, entity::ordered_ids::OrderedIds}, ComponentKind, EntityMessage, EntityMessageType, MessageIndex};
use crate::world::sync::auth_channel::AuthChannel;

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
}

impl EntityChannel {
    pub(crate) fn new() -> Self {
        Self {
            component_channels: HashMap::new(),
            outgoing_messages: Vec::new(),
            state: EntityChannelState::Despawned,
            auth_channel: AuthChannel::new(),
            buffered_messages: OrderedIds::new(),
        }
    }

    pub(crate) fn accept_message(
        &mut self,
        id: MessageIndex,
        msg: EntityMessage<()>,
    ) {
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

                    self.state = EntityChannelState::Spawned;

                    self.pop_front_into_outgoing();

                    // Drain the auth channel and append the messages to the outgoing events
                    self.auth_channel.drain_messages_into(&mut self.outgoing_messages);

                    // Drain the component channel and append the messages to the outgoing events
                    for (component_kind, component_channel) in self.component_channels.iter_mut() {
                        component_channel.drain_messages_into(component_kind, &mut self.outgoing_messages);
                    }
                },
                EntityMessageType::DespawnEntity => {
                    if self.state != EntityChannelState::Spawned {
                        break;
                    }

                    self.state = EntityChannelState::Despawned;
                    
                    self.pop_front_into_outgoing();
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