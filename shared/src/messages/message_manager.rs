use std::{collections::HashMap, hash::Hash};

use naia_serde::{BitReader, BitWrite, BitWriter, ConstBitLength, Serde, SerdeErr};
use naia_socket_shared::Instant;

use crate::{
    constants::FRAGMENTATION_LIMIT_BITS,
    messages::{
        channels::{
            channel::ChannelMode,
            channel::ChannelSettings,
            channel_kinds::{ChannelKind, ChannelKinds},
            receivers::{
                channel_receiver::MessageChannelReceiver,
                ordered_reliable_receiver::OrderedReliableReceiver,
                sequenced_reliable_receiver::SequencedReliableReceiver,
                sequenced_unreliable_receiver::SequencedUnreliableReceiver,
                unordered_reliable_receiver::UnorderedReliableReceiver,
                unordered_unreliable_receiver::UnorderedUnreliableReceiver,
            },
            senders::{
                channel_sender::MessageChannelSender, message_fragmenter::MessageFragmenter,
                reliable_message_sender::ReliableMessageSender, request_sender::LocalResponseId,
                sequenced_unreliable_sender::SequencedUnreliableSender,
                unordered_unreliable_sender::UnorderedUnreliableSender,
            },
        },
        error::MessageManagerError,
        message_container::MessageContainer,
        request::GlobalRequestId,
    },
    types::{HostType, MessageIndex, PacketIndex},
    world::{
        entity::entity_converters::LocalEntityAndGlobalEntityConverterMut,
        remote::entity_waitlist::EntityWaitlist,
    },
    EntityAndGlobalEntityConverter, EntityAndLocalEntityConverter, EntityConverter, MessageKinds,
    Protocol,
};

/// Handles incoming/outgoing messages, tracks the delivery status of Messages
/// so that guaranteed Messages can be re-transmitted to the remote host
pub struct MessageManager {
    channel_senders: HashMap<ChannelKind, Box<dyn MessageChannelSender>>,
    channel_receivers: HashMap<ChannelKind, Box<dyn MessageChannelReceiver>>,
    channel_settings: HashMap<ChannelKind, ChannelSettings>,
    packet_to_message_map: HashMap<PacketIndex, Vec<(ChannelKind, Vec<MessageIndex>)>>,
    message_fragmenter: MessageFragmenter,
}

impl MessageManager {
    /// Creates a new MessageManager
    pub fn new(host_type: HostType, channel_kinds: &ChannelKinds) -> Self {
        // initialize all reliable channels

        // initialize senders
        let mut channel_senders = HashMap::<ChannelKind, Box<dyn MessageChannelSender>>::new();
        for (channel_kind, channel_settings) in channel_kinds.channels() {
            //info!("initialize senders for channel: {:?}", channel_kind);
            match &host_type {
                HostType::Server => {
                    if !channel_settings.can_send_to_client() {
                        continue;
                    }
                }
                HostType::Client => {
                    if !channel_settings.can_send_to_server() {
                        continue;
                    }
                }
            }

            match &channel_settings.mode {
                ChannelMode::UnorderedUnreliable => {
                    channel_senders
                        .insert(channel_kind, Box::new(UnorderedUnreliableSender::new()));
                }
                ChannelMode::SequencedUnreliable => {
                    channel_senders
                        .insert(channel_kind, Box::new(SequencedUnreliableSender::new()));
                }
                ChannelMode::UnorderedReliable(settings)
                | ChannelMode::SequencedReliable(settings)
                | ChannelMode::OrderedReliable(settings) => {
                    channel_senders.insert(
                        channel_kind,
                        Box::new(ReliableMessageSender::new(settings.rtt_resend_factor)),
                    );
                }
                ChannelMode::TickBuffered(_) => {
                    // Tick buffered channel uses another manager, skip
                }
            };
        }

        // initialize receivers
        let mut channel_receivers = HashMap::<ChannelKind, Box<dyn MessageChannelReceiver>>::new();
        for (channel_kind, channel_settings) in channel_kinds.channels() {
            match &host_type {
                HostType::Server => {
                    if !channel_settings.can_send_to_server() {
                        continue;
                    }
                }
                HostType::Client => {
                    if !channel_settings.can_send_to_client() {
                        continue;
                    }
                }
            }

            match &channel_settings.mode {
                ChannelMode::UnorderedUnreliable => {
                    channel_receivers.insert(
                        channel_kind.clone(),
                        Box::new(UnorderedUnreliableReceiver::new()),
                    );
                }
                ChannelMode::SequencedUnreliable => {
                    channel_receivers.insert(
                        channel_kind.clone(),
                        Box::new(SequencedUnreliableReceiver::new()),
                    );
                }
                ChannelMode::UnorderedReliable(_) => {
                    channel_receivers.insert(
                        channel_kind.clone(),
                        Box::new(UnorderedReliableReceiver::new()),
                    );
                }
                ChannelMode::SequencedReliable(_) => {
                    channel_receivers.insert(
                        channel_kind.clone(),
                        Box::new(SequencedReliableReceiver::new()),
                    );
                }
                ChannelMode::OrderedReliable(_) => {
                    channel_receivers.insert(
                        channel_kind.clone(),
                        Box::new(OrderedReliableReceiver::new()),
                    );
                }
                ChannelMode::TickBuffered(_) => {
                    // Tick buffered channel uses another manager, skip
                }
            };
        }

        // initialize settings
        let mut channel_settings_map = HashMap::new();
        for (channel_kind, channel_settings) in channel_kinds.channels() {
            channel_settings_map.insert(channel_kind.clone(), channel_settings);
        }

        Self {
            channel_senders,
            channel_receivers,
            channel_settings: channel_settings_map,
            packet_to_message_map: HashMap::new(),
            message_fragmenter: MessageFragmenter::new(),
        }
    }

    // Outgoing Messages

    /// Queues a Message to be transmitted to the remote host (fallible version)
    pub fn try_send_message(
        &mut self,
        message_kinds: &MessageKinds,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
        channel_kind: &ChannelKind,
        message: MessageContainer,
    ) -> Result<(), MessageManagerError> {
        let channel = self.channel_senders.get_mut(channel_kind).ok_or_else(|| {
            MessageManagerError::ChannelNotConfiguredForSending {
                channel: format!("{:?}", channel_kind),
            }
        })?;

        let message_bit_length = message.bit_length();
        if message_bit_length > FRAGMENTATION_LIMIT_BITS {
            let settings = self.channel_settings.get(channel_kind).ok_or_else(|| {
                MessageManagerError::ChannelSettingsNotFound {
                    channel: format!("{:?}", channel_kind),
                }
            })?;
            if !settings.reliable() {
                return Err(MessageManagerError::FragmentationLimitExceeded {
                    bit_length: message_bit_length,
                    limit: FRAGMENTATION_LIMIT_BITS,
                    channel: format!("{:?}", channel_kind),
                });
            }

            // Now fragment this message ...
            let messages =
                self.message_fragmenter
                    .fragment_message(message_kinds, converter, message);
            for message_fragment in messages {
                channel.send_message(message_fragment);
            }
        } else {
            channel.send_message(message);
        }
        Ok(())
    }

    /// Queues an Message to be transmitted to the remote host
    pub fn send_message(
        &mut self,
        message_kinds: &MessageKinds,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
        channel_kind: &ChannelKind,
        message: MessageContainer,
    ) {
        self.try_send_message(message_kinds, converter, channel_kind, message)
            .expect("Channel not configured correctly! Cannot send message.")
    }

    pub fn try_send_request(
        &mut self,
        message_kinds: &MessageKinds,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
        channel_kind: &ChannelKind,
        global_request_id: GlobalRequestId,
        request: MessageContainer,
    ) -> Result<(), MessageManagerError> {
        let channel = self.channel_senders.get_mut(channel_kind).ok_or_else(|| {
            MessageManagerError::ChannelNotConfiguredForSending {
                channel: format!("{:?}", channel_kind),
            }
        })?;
        channel.send_outgoing_request(message_kinds, converter, global_request_id, request);
        Ok(())
    }

    pub fn send_request(
        &mut self,
        message_kinds: &MessageKinds,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
        channel_kind: &ChannelKind,
        global_request_id: GlobalRequestId,
        request: MessageContainer,
    ) {
        self.try_send_request(message_kinds, converter, channel_kind, global_request_id, request)
            .expect("Channel not configured correctly! Cannot send message.")
    }

    pub fn try_send_response(
        &mut self,
        message_kinds: &MessageKinds,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
        channel_kind: &ChannelKind,
        local_response_id: LocalResponseId,
        response: MessageContainer,
    ) -> Result<(), MessageManagerError> {
        let channel = self.channel_senders.get_mut(channel_kind).ok_or_else(|| {
            MessageManagerError::ChannelNotConfiguredForSending {
                channel: format!("{:?}", channel_kind),
            }
        })?;
        channel.send_outgoing_response(message_kinds, converter, local_response_id, response);
        Ok(())
    }

    pub fn send_response(
        &mut self,
        message_kinds: &MessageKinds,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
        channel_kind: &ChannelKind,
        local_response_id: LocalResponseId,
        response: MessageContainer,
    ) {
        self.try_send_response(message_kinds, converter, channel_kind, local_response_id, response)
            .expect("Channel not configured correctly! Cannot send message.")
    }

    pub fn collect_outgoing_messages(&mut self, now: &Instant, rtt_millis: &f32) {
        for channel in self.channel_senders.values_mut() {
            channel.collect_messages(now, rtt_millis);
        }
    }

    /// Returns whether the Manager has queued Messages that can be transmitted
    /// to the remote host
    pub fn has_outgoing_messages(&self) -> bool {
        for channel in self.channel_senders.values() {
            if channel.has_messages() {
                return true;
            }
        }
        false
    }

    pub fn write_messages(
        &mut self,
        protocol: &Protocol,
        converter: &mut dyn LocalEntityAndGlobalEntityConverterMut,
        writer: &mut BitWriter,
        packet_index: PacketIndex,
        has_written: &mut bool,
    ) {
        for (channel_kind, channel) in &mut self.channel_senders {
            if !channel.has_messages() {
                continue;
            }

            // check that we can at least write a ChannelIndex and a MessageContinue bit
            let mut counter = writer.counter();
            // reserve MessageContinue bit
            counter.write_bit(false);
            // write ChannelContinue bit
            counter.write_bit(false);
            // write ChannelIndex
            counter.count_bits(<ChannelKind as ConstBitLength>::const_bit_length());
            if counter.overflowed() {
                break;
            }

            // reserve MessageContinue bit
            writer.reserve_bits(1);
            // write ChannelContinue bit
            true.ser(writer);
            // write ChannelIndex
            channel_kind.ser(&protocol.channel_kinds, writer);
            // write Messages
            if let Some(message_indices) =
                channel.write_messages(&protocol.message_kinds, converter, writer, has_written)
            {
                let channel_list = self
                    .packet_to_message_map
                    .entry(packet_index)
                    .or_insert_with(Vec::new);
                channel_list.push((channel_kind.clone(), message_indices));
            }

            // write MessageContinue finish bit, release
            writer.release_bits(1);
            false.ser(writer);
        }

        // write ChannelContinue finish bit, release
        writer.release_bits(1);
        false.ser(writer);
    }

    // Incoming Messages

    pub fn try_read_messages<E: Copy + Eq + Hash + Send + Sync>(
        &mut self,
        protocol: &Protocol,
        entity_waitlist: &mut EntityWaitlist,
        global_converter: &dyn EntityAndGlobalEntityConverter<E>,
        local_converter: &dyn EntityAndLocalEntityConverter<E>,
        reader: &mut BitReader,
    ) -> Result<(), MessageManagerError> {
        let converter = EntityConverter::new(global_converter, local_converter);
        loop {
            let message_continue = bool::de(reader).map_err(|_| {
                MessageManagerError::ChannelNotConfiguredForReceiving {
                    channel: "unknown".to_string(),
                }
            })?;
            if !message_continue {
                break;
            }

            // read channel id
            let channel_kind = ChannelKind::de(&protocol.channel_kinds, reader).map_err(|_| {
                MessageManagerError::ChannelNotConfiguredForReceiving {
                    channel: "unknown".to_string(),
                }
            })?;

            // continue read inside channel
            let channel = self.channel_receivers.get_mut(&channel_kind).ok_or_else(|| {
                MessageManagerError::ChannelNotConfiguredForReceiving {
                    channel: format!("{:?}", channel_kind),
                }
            })?;
            channel
                .read_messages(&protocol.message_kinds, entity_waitlist, &converter, reader)
                .map_err(|_| MessageManagerError::ChannelNotConfiguredForReceiving {
                    channel: format!("{:?}", channel_kind),
                })?;
        }

        Ok(())
    }

    pub fn read_messages<E: Copy + Eq + Hash + Send + Sync>(
        &mut self,
        protocol: &Protocol,
        entity_waitlist: &mut EntityWaitlist,
        global_converter: &dyn EntityAndGlobalEntityConverter<E>,
        local_converter: &dyn EntityAndLocalEntityConverter<E>,
        reader: &mut BitReader,
    ) -> Result<(), SerdeErr> {
        self.try_read_messages(protocol, entity_waitlist, global_converter, local_converter, reader)
            .map_err(|_| SerdeErr)
    }

    /// Retrieve all messages from the channel buffers
    pub fn receive_messages<E: Eq + Copy + Hash>(
        &mut self,
        message_kinds: &MessageKinds,
        now: &Instant,
        global_entity_converter: &dyn EntityAndGlobalEntityConverter<E>,
        local_entity_converter: &dyn EntityAndLocalEntityConverter<E>,
        entity_waitlist: &mut EntityWaitlist,
    ) -> Vec<(ChannelKind, Vec<MessageContainer>)> {
        let entity_converter =
            EntityConverter::new(global_entity_converter, local_entity_converter);
        let mut output = Vec::new();
        // TODO: shouldn't we have a priority mechanisms between channels?
        for (channel_kind, channel) in &mut self.channel_receivers {
            let messages =
                channel.receive_messages(message_kinds, now, entity_waitlist, &entity_converter);
            output.push((channel_kind.clone(), messages));
        }
        output
    }

    /// Retrieve all requests and responses from the channel buffers (fallible version)
    pub fn try_receive_requests_and_responses(
        &mut self,
    ) -> Result<
        (
            Vec<(ChannelKind, Vec<(LocalResponseId, MessageContainer)>)>,
            Vec<(GlobalRequestId, MessageContainer)>,
        ),
        MessageManagerError,
    > {
        let mut request_output = Vec::new();
        let mut response_output = Vec::new();
        for (channel_kind, channel) in &mut self.channel_receivers {
            let settings = self.channel_settings.get(channel_kind).ok_or_else(|| {
                MessageManagerError::ChannelSettingsNotFound {
                    channel: format!("{:?}", channel_kind),
                }
            })?;
            if !settings.can_request_and_respond() {
                continue;
            }

            let (requests, responses) = channel.receive_requests_and_responses();
            if !requests.is_empty() {
                request_output.push((channel_kind.clone(), requests));
            }

            if !responses.is_empty() {
                let channel_sender = self.channel_senders.get_mut(channel_kind).ok_or_else(|| {
                    MessageManagerError::ChannelNotConfiguredForSending {
                        channel: format!("{:?}", channel_kind),
                    }
                })?;
                for (local_request_id, response) in responses {
                    let global_request_id = channel_sender
                        .process_incoming_response(&local_request_id)
                        .ok_or(MessageManagerError::ResponseProcessingFailed)?;
                    response_output.push((global_request_id, response));
                }
            }
        }
        Ok((request_output, response_output))
    }

    /// Retrieve all requests from the channel buffers
    pub fn receive_requests_and_responses(
        &mut self,
    ) -> (
        Vec<(ChannelKind, Vec<(LocalResponseId, MessageContainer)>)>,
        Vec<(GlobalRequestId, MessageContainer)>,
    ) {
        self.try_receive_requests_and_responses()
            .expect("Channel not configured correctly! Cannot process requests/responses.")
    }
}

impl MessageManager {
    /// Occurs when a packet has been notified as delivered. Stops tracking the
    /// status of Messages in that packet.
    pub fn notify_packet_delivered(&mut self, packet_index: PacketIndex) {
        if let Some(channel_list) = self.packet_to_message_map.get(&packet_index) {
            for (channel_kind, message_indices) in channel_list {
                if let Some(channel) = self.channel_senders.get_mut(channel_kind) {
                    for message_index in message_indices {
                        channel.notify_message_delivered(message_index);
                    }
                }
            }
        }
    }
}
