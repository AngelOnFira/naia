use std::{collections::VecDeque, mem};

use naia_serde::{BitReader, Serde, SerdeErr};
use naia_socket_shared::Instant;

use crate::messages::channels::senders::request_sender::LocalRequestId;
use crate::{
    messages::{
        channels::receivers::{
            channel_receiver::{ChannelReceiver, MessageChannelReceiver},
            error::ReceiverError,
        },
        message_kinds::MessageKinds,
    },
    world::remote::entity_waitlist::{EntityWaitlist, WaitlistStore},
    LocalEntityAndGlobalEntityConverter, LocalResponseId, MessageContainer,
};

pub struct UnorderedUnreliableReceiver {
    incoming_messages: VecDeque<MessageContainer>,
    waitlist_store: WaitlistStore<MessageContainer>,
}

impl UnorderedUnreliableReceiver {
    pub fn new() -> Self {
        Self {
            incoming_messages: VecDeque::new(),
            waitlist_store: WaitlistStore::new(),
        }
    }

    fn read_message(
        &mut self,
        message_kinds: &MessageKinds,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        reader: &mut BitReader,
    ) -> Result<MessageContainer, SerdeErr> {
        // read payload
        message_kinds.read(reader, converter)
    }

    fn recv_message(&mut self, entity_waitlist: &mut EntityWaitlist, message: MessageContainer) {
        if let Some(entity_set) = message.relations_waiting() {
            entity_waitlist.queue(&entity_set, &mut self.waitlist_store, message);
            return;
        }

        self.incoming_messages.push_back(message);
    }
}

impl ChannelReceiver<MessageContainer> for UnorderedUnreliableReceiver {
    fn receive_messages(
        &mut self,
        _message_kinds: &MessageKinds,
        now: &Instant,
        entity_waitlist: &mut EntityWaitlist,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
    ) -> Vec<MessageContainer> {
        if let Some(list) = entity_waitlist.collect_ready_items(now, &mut self.waitlist_store) {
            for mut message in list {
                message.relations_complete(converter);
                self.incoming_messages.push_back(message);
            }
        }

        Vec::from(mem::take(&mut self.incoming_messages))
    }
}

impl MessageChannelReceiver for UnorderedUnreliableReceiver {
    fn read_messages(
        &mut self,
        message_kinds: &MessageKinds,
        entity_waitlist: &mut EntityWaitlist,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        reader: &mut BitReader,
    ) -> Result<(), SerdeErr> {
        loop {
            let channel_continue = bool::de(reader)?;
            if !channel_continue {
                break;
            }

            let message = self.read_message(message_kinds, converter, reader)?;
            self.recv_message(entity_waitlist, message);
        }

        Ok(())
    }

    fn receive_requests_and_responses(
        &mut self,
    ) -> (
        Vec<(LocalResponseId, MessageContainer)>,
        Vec<(LocalRequestId, MessageContainer)>,
    ) {
        Self::try_receive_requests_and_responses()
            .unwrap_or_else(|e| panic!("UnorderedUnreliableReceiver error: {}", e))
    }
}

impl UnorderedUnreliableReceiver {
    /// Attempt to receive requests and responses (not supported on this channel type)
    ///
    /// Returns Err always since UnorderedUnreliable channels do not support requests
    pub fn try_receive_requests_and_responses() -> Result<(
        Vec<(LocalResponseId, MessageContainer)>,
        Vec<(LocalRequestId, MessageContainer)>,
    ), ReceiverError> {
        Err(ReceiverError::RequestsNotSupported {
            channel_type: "UnorderedUnreliable",
        })
    }
}
