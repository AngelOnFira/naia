use std::{hash::Hash, collections::HashMap};

use crate::{world::{sync::component_channel::ComponentChannel, entity::ordered_ids::OrderedIds}, ComponentKind, EntityMessage, EntityMessageType, MessageIndex};

pub(crate) struct EntityChannel {
    component_channels: HashMap<ComponentKind, ComponentChannel>,
    outgoing_messages: Vec<EntityMessage<()>>,
    spawned: bool,
    buffered_messages: OrderedIds<EntityMessage<()>>,
}

impl EntityChannel {
    pub(crate) fn new() -> Self {
        Self {
            component_channels: HashMap::new(),
            outgoing_messages: Vec::new(),
            spawned: false,
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
                    if self.spawned {
                        break;
                    }

                    self.spawned = true;

                    let (_, msg) = self.buffered_messages.pop_front().unwrap();
                    self.outgoing_messages.push(msg);

                    // Drain the component channel and append the messages to the outgoing events
                    for (component_kind, component_channel) in self.component_channels.iter_mut() {
                        component_channel.drain_messages_into(component_kind, &mut self.outgoing_messages);
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

                    component_channel.drain_messages_into(&component_kind, &mut self.outgoing_messages);
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

                    component_channel.drain_messages_into(&component_kind, &mut self.outgoing_messages);
                }
                _ => {
                    todo!();
                }
            }
        }
    }
}