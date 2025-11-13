use std::collections::HashMap;

use naia_serde::BitReader;

use crate::{
    messages::{
        channels::receivers::error::ReceiverError,
        fragment::{FragmentId, FragmentIndex, FragmentedMessage},
    },
    LocalEntityAndGlobalEntityConverter, MessageContainer, MessageIndex, MessageKinds,
};

pub struct FragmentReceiver {
    // <FragmentId, (FragmentsReceived, Option(FirstMessageIndex, FragmentCount), FragmentData)
    map: HashMap<FragmentId, (u32, Option<(MessageIndex, u32)>, Vec<Box<[u8]>>)>,
}

impl FragmentReceiver {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Attempt to receive and reassemble a fragmented message
    ///
    /// Returns Ok(None) if more fragments are needed, Ok(Some(...)) if message is complete,
    /// or Err if the message is invalid or cannot be reassembled
    pub(crate) fn try_receive(
        &mut self,
        message_kinds: &MessageKinds,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        message_index: MessageIndex,
        message: MessageContainer,
    ) -> Result<Option<(MessageIndex, MessageIndex, MessageContainer)>, ReceiverError> {
        if !message.is_fragment() {
            return Err(ReceiverError::NonFragmentedMessage);
        }

        // Message is a fragment, need to process
        let fragment = message
            .to_boxed_any()
            .downcast::<FragmentedMessage>()
            .map_err(|_| ReceiverError::MessageDowncastFailed {
                expected_type: "FragmentedMessage",
            })?;
        let fragment_id = fragment.id();
        let fragment_index = fragment.index();
        let fragment_total = fragment.total().as_usize();

        if !self.map.contains_key(&fragment_id) {
            self.map
                .insert(fragment_id, (0, None, vec![Box::new([]); fragment_total]));
        }
        let (fragments_received, first_message_id_opt, fragment_list) =
            self.map.get_mut(&fragment_id)
                .ok_or(ReceiverError::FragmentIdNotFound)?;

        if fragment_index == FragmentIndex::zero() {
            if first_message_id_opt.is_some() {
                return Err(ReceiverError::DuplicateFirstFragment);
            }
            *first_message_id_opt = Some((message_index, fragment_total as u32));
        }

        fragment_list[fragment_index.as_usize()] = fragment.to_payload();
        *fragments_received += 1;
        if *fragments_received != fragment_total as u32 {
            return Ok(None);
        }

        // we have received all fragments! put it all together
        let (_, first_index_opt, fragment_list) = self.map.remove(&fragment_id)
            .ok_or(ReceiverError::FragmentIdNotFound)?;
        let (first_message_index, fragment_count) = first_index_opt
            .ok_or(ReceiverError::FirstFragmentMetadataMissing)?;
        let concat_list = fragment_list.concat();
        let mut reader = BitReader::new(&concat_list);
        let full_message = message_kinds.read(&mut reader, converter)
            .map_err(|_| ReceiverError::FragmentedMessageReadFailed {
                reason: "deserialization failed",
            })?;
        let end_message_index = first_message_index + fragment_count as u16 - 1;
        Ok(Some((first_message_index, end_message_index, full_message)))
    }

    /// Receive and reassemble a fragmented message (backward compatible)
    ///
    /// # Panics
    /// Panics if the message is invalid or cannot be reassembled.
    /// For non-panicking version, use `try_receive`.
    pub(crate) fn receive(
        &mut self,
        message_kinds: &MessageKinds,
        converter: &dyn LocalEntityAndGlobalEntityConverter,
        message_index: MessageIndex,
        message: MessageContainer,
    ) -> Option<(MessageIndex, MessageIndex, MessageContainer)> {
        self.try_receive(message_kinds, converter, message_index, message)
            .unwrap_or_else(|e| panic!("FragmentReceiver error: {}", e))
    }
}
