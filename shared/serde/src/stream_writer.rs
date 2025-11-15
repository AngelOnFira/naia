use crate::BitWrite;

/// A BitWrite implementation that can grow beyond MTU size for QUIC streams.
/// Unlike BitWriter which has a fixed MTU-sized buffer, StreamWriter uses
/// a Vec<u8> that can grow to accommodate large messages.
pub struct StreamWriter {
    scratch: u8,
    scratch_index: u8,
    buffer: Vec<u8>,
    bits_written: u32,
}

impl StreamWriter {
    pub fn new() -> Self {
        Self {
            scratch: 0,
            scratch_index: 0,
            buffer: Vec::with_capacity(4096), // Start with 4KB, will grow as needed
            bits_written: 0,
        }
    }

    fn flush_scratch(&mut self) {
        if self.scratch_index > 0 {
            let byte = (self.scratch << (8 - self.scratch_index)).reverse_bits();
            self.buffer.push(byte);
            self.scratch = 0;
            self.scratch_index = 0;
        }
    }

    pub fn to_bytes(mut self) -> Vec<u8> {
        self.flush_scratch();
        self.buffer
    }

    pub fn bits_written(&self) -> u32 {
        self.bits_written
    }
}

impl BitWrite for StreamWriter {
    fn write_bit(&mut self, bit: bool) {
        self.scratch <<= 1;

        if bit {
            self.scratch |= 1;
        }

        self.scratch_index += 1;
        self.bits_written += 1;

        if self.scratch_index >= 8 {
            self.buffer.push(self.scratch.reverse_bits());
            self.scratch_index = 0;
            self.scratch = 0;
        }
    }

    fn write_byte(&mut self, byte: u8) {
        let mut temp = byte;
        for _ in 0..8 {
            self.write_bit(temp & 1 != 0);  // Read LSB first (match BitWriter)
            temp >>= 1;                      // Shift right (match BitWriter)
        }
    }

    fn is_counter(&self) -> bool {
        false
    }

    fn count_bits(&mut self, _bits: u32) {
        // StreamWriter doesn't need counting - it can grow indefinitely
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_writer_basic() {
        let mut writer = StreamWriter::new();

        // Write a byte
        writer.write_byte(0b10101010);

        let bytes = writer.to_bytes();
        assert_eq!(bytes.len(), 1);
        assert_eq!(bytes[0], 0b10101010);
    }

    #[test]
    fn test_stream_writer_large() {
        let mut writer = StreamWriter::new();

        // Write 10KB of data
        for _ in 0..10_000 {
            writer.write_byte(0xFF);
        }

        let bytes = writer.to_bytes();
        assert_eq!(bytes.len(), 10_000);
        assert!(bytes.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_stream_writer_bits() {
        let mut writer = StreamWriter::new();

        // Write individual bits (LSB first, like BitWriter)
        writer.write_bit(false);  // bit 0
        writer.write_bit(true);   // bit 1
        writer.write_bit(false);  // bit 2
        writer.write_bit(true);   // bit 3
        writer.write_bit(false);  // bit 4
        writer.write_bit(true);   // bit 5
        writer.write_bit(false);  // bit 6
        writer.write_bit(true);   // bit 7

        let bytes = writer.to_bytes();
        assert_eq!(bytes.len(), 1);
        assert_eq!(bytes[0], 0b10101010);
    }

    #[test]
    fn test_stream_writer_matches_bit_writer() {
        use crate::{BitWrite, BitWriter};

        // Test that StreamWriter produces the same output as BitWriter for the same inputs
        let test_data: Vec<u8> = vec![0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xEF];

        let mut stream_writer = StreamWriter::new();
        let mut bit_writer = BitWriter::new();

        for &byte in &test_data {
            stream_writer.write_byte(byte);
            bit_writer.write_byte(byte);
        }

        let stream_bytes = stream_writer.to_bytes();
        let bit_bytes = bit_writer.to_bytes();

        assert_eq!(stream_bytes.len(), bit_bytes.len(), "Byte count should match");
        assert_eq!(&stream_bytes[..], &bit_bytes[..], "Byte content should match");
    }
}
