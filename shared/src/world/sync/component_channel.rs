use crate::{EntityMessage, EntityMessageType, MessageIndex};
use crate::world::entity::ordered_ids::OrderedIds;

pub struct ComponentChannel {
    outgoing_messages: Vec<EntityMessageType>,
    inserted: bool,
    buffered_messages: OrderedIds<bool>,
}

impl ComponentChannel {
    pub fn new() -> Self {
        Self {
            outgoing_messages: Vec::new(),
            inserted: false,
            buffered_messages: OrderedIds::new(),
        }
    }

    pub(crate) fn accept_message(&mut self, id: MessageIndex, msg: EntityMessage<()>) {
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
            let Some((_, insert)) = self.buffered_messages.peek_front() else {
                break;
            };
            let mut pop = false;

            if *insert {
                if self.inserted {
                    break;
                }
                self.inserted = true;
                pop = true;
            } else {
                if !self.inserted {
                    break;
                }
                self.inserted = false;
                pop = true;
            }

            if pop {
                let (_, insert) = self.buffered_messages.pop_front().unwrap();
                if insert {
                    self.outgoing_messages.push(EntityMessageType::InsertComponent);
                } else {
                    self.outgoing_messages.push(EntityMessageType::RemoveComponent);
                }
            } else {
                panic!("should pop if we reach here");
            }
        }
    }
    
    pub fn receive_messages(&mut self) -> Vec<EntityMessageType> {
        std::mem::take(&mut self.outgoing_messages)
    }
}