use crate::{sequence_equal_or_less_than, world::entity::ordered_ids::OrderedIds, ComponentKind, EntityMessage, EntityMessageType, MessageIndex};

pub(crate) struct ComponentChannel {
    outgoing_messages: Vec<EntityMessageType>,
    inserted: bool,
    buffered_messages: OrderedIds<bool>,
    last_insert_id: Option<MessageIndex>,
}

impl ComponentChannel {
    pub(crate) fn new() -> Self {
        Self {
            outgoing_messages: Vec::new(),
            inserted: false,
            buffered_messages: OrderedIds::new(),
            last_insert_id: None,
        }
    }

    pub(crate) fn drain_messages_into(
        &mut self,
        component_kind: &ComponentKind,
        outgoing_messages: &mut Vec<EntityMessage<()>>,
    ) {
        // Drain the component channel and append the messages to the outgoing events
        let mut received_messages = Vec::new();
        for msg_type in std::mem::take(&mut self.outgoing_messages) {
            received_messages.push(msg_type.with_component_kind(&component_kind));
        }
        outgoing_messages.append(&mut received_messages);
    }

    pub(crate) fn buffer_pop_front_until_and_excluding(&mut self, id: MessageIndex) {
        self.buffered_messages.pop_front_until_and_excluding(id);
    }

    pub(crate) fn accept_message(&mut self, id: MessageIndex, msg: EntityMessage<()>) {

        if let Some(last_insert_id) = self.last_insert_id {
            if sequence_equal_or_less_than(id, last_insert_id) {
                // This message is older than the last insert message, ignore it
                return;
            }
        }

        let insert = match &msg {
            EntityMessage::InsertComponent(_, _) => true,
            EntityMessage::RemoveComponent(_, _) => false,
            _ => panic!("ComponentChannel can only accept InsertComponent or RemoveComponent messages"),
        };

        self.buffered_messages.push_back(id, insert);
        
        self.process_messages();
    }
    
    fn process_messages(&mut self) {
        loop {
            let Some((id, insert)) = self.buffered_messages.peek_front() else {
                break;
            };

            let id = *id;

            match *insert {
                true => {
                    if self.inserted {
                        break;
                    }
                    self.inserted = true;
                    self.last_insert_id = Some(id);
                }
                false => {
                    if !self.inserted {
                        break;
                    }
                    self.inserted = false;
                    self.last_insert_id = None;
                }
            }


            let (_, insert) = self.buffered_messages.pop_front().unwrap();
            if insert {
                self.outgoing_messages.push(EntityMessageType::InsertComponent);
            } else {
                self.outgoing_messages.push(EntityMessageType::RemoveComponent);
            }
        }
    }
}