use std::collections::HashMap;

use crate::{ComponentKind, EntityMessage, EntityMessageType, MessageIndex};
use crate::world::entity::ordered_ids::OrderedIds;
use crate::world::sync::component_channel::ComponentChannel;

pub struct EntityChannel {
    component_channels: HashMap<ComponentKind, ComponentChannel>,
    outgoing_messages: Vec<EntityMessage<()>>,
    spawned: bool,
    buffered_messages: OrderedIds<EntityMessage<()>>,
}

impl EntityChannel {
    pub fn new() -> Self {
        Self {
            component_channels: HashMap::new(),
            outgoing_messages: Vec::new(),
            spawned: false,
            buffered_messages: OrderedIds::new(),
        }
    }

    pub fn accept_message(
        &mut self,
        id: MessageIndex,
        msg: EntityMessage<()>,
    ) {
        self.buffered_messages.push_back(id, msg);

        self.process_messages();
    }

    pub fn receive_messages(&mut self) -> Vec<EntityMessage<()>> {
        std::mem::take(&mut self.outgoing_messages)
    }

    fn process_messages(&mut self) {
        loop {
            let Some((id, msg)) = self.buffered_messages.peek_front() else {
                break;
            };
            match msg.get_type() {
                EntityMessageType::SpawnEntity => {
                    if self.spawned {
                        break;
                    }

                    self.spawned = true;

                    let (_, msg) = self.buffered_messages.pop_front().unwrap();
                    self.outgoing_messages.push(msg);

                    // Drain the component channel and append the messages to the outgoing events
                    for (component_kind, component_channel) in self.component_channels.iter_mut() {
                        Self::drain_component_messages(component_kind, component_channel, &mut self.outgoing_messages);
                    }
            },
            EntityMessageType::DespawnEntity => {
                    if !self.spawned {
                        break;
                    }

                    self.spawned = false;
                    
                    let (_, msg) = self.buffered_messages.pop_front().unwrap();
                    self.outgoing_messages.push(msg);
                },
                EntityMessageType::InsertComponent => {

                    let (id, msg) = self.buffered_messages.pop_front().unwrap();
                    
                    let component_kind = msg.component_kind().unwrap();
                    let component_channel = self.component_channels
                        .entry(component_kind)
                        .or_insert_with(ComponentChannel::new);

                    component_channel.accept_message(id, msg);
                    
                    if !self.spawned {
                        continue;
                    }

                    Self::drain_component_messages(&component_kind, component_channel, &mut self.outgoing_messages);
                }
                EntityMessageType::RemoveComponent => {

                    let (id, msg) = self.buffered_messages.pop_front().unwrap();

                    let component_kind = msg.component_kind().unwrap();
                    let component_channel = self.component_channels
                        .entry(component_kind)
                        .or_insert_with(ComponentChannel::new);

                    component_channel.accept_message(id, msg);

                    if !self.spawned {
                        continue;
                    }

                    Self::drain_component_messages(&component_kind, component_channel, &mut self.outgoing_messages);
                }
                _ => {
                    todo!();
                }
            }
        }
    }

    fn drain_component_messages(component_kind: &ComponentKind, component_channel: &mut ComponentChannel, outgoing_messages: &mut Vec<EntityMessage<()>>) {
        // Drain the component channel and append the messages to the outgoing events
        let mut received_messages = Vec::new();
        for msg_type in component_channel.receive_messages() {
            received_messages.push(msg_type.with_component_kind(&component_kind));
        }
        outgoing_messages.append(&mut received_messages);
    }
}