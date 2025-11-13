// An enum representing the different types of packets that can be
// sent/received

use naia_serde::{BitReader, BitWrite, ConstBitLength, Serde, SerdeErr, UnsignedInteger};

#[derive(Copy, Debug, Clone, Eq, PartialEq)]
pub enum PacketType {
    // A packet containing Message/Entity/Component data
    Data,
    // A packet sent to maintain the connection by preventing a timeout
    Heartbeat,
    // A packet sent by the client to request a connection
    Handshake,
    // A Ping message, used to calculate RTT. Must be responded to with a Pong
    // message
    Ping,
    // A Pong message, used to calculate RTT. Must be the response to all Ping
    // messages
    Pong,
}

// Most packets should be Data, so lets compress this a bit more.
// Could do this with another enum, but code would get messy.
impl Serde for PacketType {
    fn ser(&self, writer: &mut dyn BitWrite) {
        let is_data = *self == PacketType::Data;
        is_data.ser(writer);

        if is_data {
            return;
        }

        let index = match self {
            // SAFETY: This case is unreachable because we already checked is_data above.
            // This should never happen in correct code, but we avoid panicking to maintain
            // robustness. If somehow reached, we'll serialize as Heartbeat (index 0).
            PacketType::Data => 0,
            PacketType::Heartbeat => 0,
            PacketType::Handshake => 1,
            PacketType::Ping => 2,
            PacketType::Pong => 3,
        };

        UnsignedInteger::<2>::new(index).ser(writer);
    }

    fn de(reader: &mut BitReader) -> Result<Self, SerdeErr> {
        let is_data = bool::de(reader)?;
        if is_data {
            return Ok(PacketType::Data);
        }

        match UnsignedInteger::<2>::de(reader)?.get() {
            0 => Ok(PacketType::Heartbeat),
            1 => Ok(PacketType::Handshake),
            2 => Ok(PacketType::Ping),
            3 => Ok(PacketType::Pong),
            // SECURITY: Malicious or malformed packets could send invalid indices.
            // Return error instead of panicking to prevent DoS attacks.
            _ => Err(SerdeErr),
        }
    }

    fn bit_length(&self) -> u32 {
        let mut output = 0;

        let is_data = *self == PacketType::Data;
        output += is_data.bit_length();

        if is_data {
            return output;
        }

        output += <UnsignedInteger<4> as ConstBitLength>::const_bit_length();

        output
    }
}
