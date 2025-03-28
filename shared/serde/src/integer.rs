use crate::{
    bit_reader::BitReader, bit_writer::BitWrite, error::SerdeErr, serde::Serde, ConstBitLength,
};

pub trait SerdeIntegerConversion<const SIGNED: bool, const VARIABLE: bool, const BITS: u8> {
    fn from(value: &SerdeInteger<SIGNED, VARIABLE, BITS>) -> Self;
}

pub type UnsignedInteger<const BITS: u8> = SerdeInteger<false, false, BITS>;
pub type SignedInteger<const BITS: u8> = SerdeInteger<true, false, BITS>;
pub type UnsignedVariableInteger<const BITS: u8> = SerdeInteger<false, true, BITS>;
pub type SignedVariableInteger<const BITS: u8> = SerdeInteger<true, true, BITS>;

// This outer generic type wraps an inner type that is not generic, to reduce code bloat through monomorphization.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SerdeInteger<const SIGNED: bool, const VARIABLE: bool, const BITS: u8> {
    inner: SerdeIntegerInner,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct SerdeIntegerInner {
    inner_value: i128,
    signed: bool,
    variable: bool,
    bits: u8,
}

impl SerdeIntegerInner {
    fn new(signed: bool, variable: bool, bits: u8, value: i128) -> Self {
        // replicate your original checks
        if bits == 0 {
            panic!("can't create an integer with 0 bits...");
        }
        if bits > 127 {
            panic!("can't create an integer with more than 127 bits...");
        }

        if !signed && value < 0 {
            panic!("can't encode a negative number with an Unsigned Integer!");
        }

        if !variable {
            let max_value: i128 = 2_i128.pow(bits as u32);
            if value >= max_value {
                panic!(
                    "with {} bits, can't encode number greater than {}",
                    bits, max_value
                );
            }
            if signed && value < 0 {
                let min_value: i128 = -(2_i128.pow(bits as u32));
                if value <= min_value {
                    panic!(
                        "with {} bits, can't encode number less than {}",
                        bits, min_value
                    );
                }
            }
        }

        Self {
            inner_value: value,
            signed,
            variable,
            bits,
        }
    }

    fn new_unchecked(signed: bool, variable: bool, bits: u8, value: i128) -> Self {
        Self {
            inner_value: value,
            signed,
            variable,
            bits,
        }
    }

    fn get(&self) -> i128 {
        self.inner_value
    }

    fn set(&mut self, value: i128) {
        self.inner_value = value;
    }

    fn ser(&self, writer: &mut dyn BitWrite) {
        // replicate original ser logic
        let mut value: u128;
        let negative = self.inner_value < 0;

        if self.signed {
            writer.write_bit(negative);
            if negative {
                value = -self.inner_value as u128;
            } else {
                value = self.inner_value as u128;
            }
        } else {
            value = self.inner_value as u128;
        }

        if self.variable {
            loop {
                let proceed = value >= 2_u128.pow(self.bits as u32);
                writer.write_bit(proceed);
                for _ in 0..self.bits {
                    writer.write_bit(value & 1 != 0);
                    value >>= 1;
                }
                if !proceed {
                    return;
                }
            }
        } else {
            for _ in 0..self.bits {
                writer.write_bit(value & 1 != 0);
                value >>= 1;
            }
        }
    }

    fn de(
        reader: &mut BitReader,
        signed: bool,
        variable: bool,
        bits: u8,
    ) -> Result<Self, SerdeErr> {
        let mut negative = false;
        if signed {
            negative = reader.read_bit()?;
        }

        if variable {
            let mut total_bits: usize = 0;
            let mut output: u128 = 0;

            loop {
                let proceed = reader.read_bit()?;

                for _ in 0..bits {
                    total_bits += 1;
                    output <<= 1;
                    if reader.read_bit()? {
                        output |= 1;
                    }
                }

                if !proceed {
                    output <<= 128 - total_bits;
                    output = output.reverse_bits();
                    let value: i128 = output as i128;
                    if negative {
                        return Ok(Self::new_unchecked(
                            signed, variable, bits, -value,
                        ));
                    } else {
                        return Ok(Self::new_unchecked(
                            signed, variable, bits, value,
                        ));
                    }
                }
            }
        } else {
            let mut output: u128 = 0;
            for _ in 0..bits {
                output <<= 1;
                if reader.read_bit()? {
                    output |= 1;
                }
            }
            output <<= 128 - bits;
            output = output.reverse_bits();

            let value: i128 = output as i128;
            if negative {
                Ok(Self::new_unchecked(signed, variable, bits, -value))
            } else {
                Ok(Self::new_unchecked(signed, variable, bits, value))
            }
        }
    }

    fn bit_length(&self) -> u32 {
        let mut output: u32 = 0;

        if self.signed {
            output += 1; // sign bit
        }

        if self.variable {
            let mut value = self.inner_value.abs() as u128;
            loop {
                let proceed = value >= 2_u128.pow(self.bits as u32);
                output += 1; // proceed bit
                for _ in 0..self.bits {
                    output += 1;
                    value >>= 1;
                }
                if !proceed {
                    break;
                }
            }
        } else {
            output += self.bits as u32;
        }
        output
    }
}

impl<const SIGNED: bool, const VARIABLE: bool, const BITS: u8> SerdeInteger<SIGNED, VARIABLE, BITS> {
    pub fn new<T: Into<i128>>(value: T) -> Self {
        Self {
            inner: SerdeIntegerInner::new(SIGNED, VARIABLE, BITS, value.into())
        }
    }

    pub fn get(&self) -> i128 {
        self.inner.get()
    }

    pub fn set<T: Into<i128>>(&mut self, value: T) {
        self.inner.set(value.into());
    }

    pub fn to<T: SerdeIntegerConversion<SIGNED, VARIABLE, BITS>>(&self) -> T {
        T::from(self)
    }
}

impl<const SIGNED: bool, const VARIABLE: bool, const BITS: u8> Serde for SerdeInteger<SIGNED, VARIABLE, BITS> {
    fn ser(&self, writer: &mut dyn BitWrite) {
        self.inner.ser(writer);
    }

    fn de(reader: &mut BitReader) -> Result<Self, SerdeErr> {
        let inner = SerdeIntegerInner::de(reader, SIGNED, VARIABLE, BITS)?;
        Ok(Self { inner })
    }

    fn bit_length(&self) -> u32 {
        self.inner.bit_length()
    }
}

impl<const SIGNED: bool, const BITS: u8> ConstBitLength for SerdeInteger<SIGNED, false, BITS> {
    fn const_bit_length() -> u32 {
        let mut output: u32 = 0;
        if SIGNED {
            output += 1;
        }
        output + BITS as u32
    }
}

impl<const SIGNED: bool, const VARIABLE: bool, const BITS: u8, T: Into<i128>> From<T> for SerdeInteger<SIGNED, VARIABLE, BITS> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<const SIGNED: bool, const VARIABLE: bool, const BITS: u8, T: TryFrom<i128>> SerdeIntegerConversion<SIGNED, VARIABLE, BITS> for T {
    fn from(value: &SerdeInteger<SIGNED, VARIABLE, BITS>) -> Self {
        let Ok(t_value) = T::try_from(value.inner.inner_value) else {
            panic!("SerdeInteger's value is out of range to convert to this type.");
        };
        t_value
    }
}

// Tests

#[cfg(test)]
mod tests {
    use crate::{
        bit_reader::BitReader,
        bit_writer::BitWriter,
        integer::{SignedInteger, SignedVariableInteger, UnsignedInteger, UnsignedVariableInteger},
        serde::Serde,
    };

    #[test]
    fn in_and_out() {
        let in_u16: u16 = 123;
        let middle = UnsignedInteger::<9>::new(in_u16);
        let out_u16: u16 = middle.get() as u16;

        assert_eq!(in_u16, out_u16);
    }

    #[test]
    fn read_write_unsigned() {
        // Write
        let mut writer = BitWriter::new();

        let in_1 = UnsignedInteger::<7>::new(123);
        let in_2 = UnsignedInteger::<20>::new(535221);
        let in_3 = UnsignedInteger::<2>::new(3);

        in_1.ser(&mut writer);
        in_2.ser(&mut writer);
        in_3.ser(&mut writer);

        let buffer = writer.to_bytes();

        // Read
        let mut reader = BitReader::new(&buffer);

        let out_1 = Serde::de(&mut reader).unwrap();
        let out_2 = Serde::de(&mut reader).unwrap();
        let out_3 = Serde::de(&mut reader).unwrap();

        assert_eq!(in_1, out_1);
        assert_eq!(in_2, out_2);
        assert_eq!(in_3, out_3);
    }

    #[test]
    fn read_write_signed() {
        // Write
        let mut writer = BitWriter::new();

        let in_1 = SignedInteger::<10>::new(-668);
        let in_2 = SignedInteger::<20>::new(53);
        let in_3 = SignedInteger::<2>::new(-3);

        in_1.ser(&mut writer);
        in_2.ser(&mut writer);
        in_3.ser(&mut writer);

        let buffer = writer.to_bytes();

        // Read
        let mut reader = BitReader::new(&buffer);

        let out_1 = Serde::de(&mut reader).unwrap();
        let out_2 = Serde::de(&mut reader).unwrap();
        let out_3 = Serde::de(&mut reader).unwrap();

        assert_eq!(in_1, out_1);
        assert_eq!(in_2, out_2);
        assert_eq!(in_3, out_3);
    }

    #[test]
    fn read_write_unsigned_variable() {
        // Write
        let mut writer = BitWriter::new();

        let in_1 = UnsignedVariableInteger::<3>::new(23);
        let in_2 = UnsignedVariableInteger::<5>::new(153);
        let in_3 = UnsignedVariableInteger::<2>::new(3);

        in_1.ser(&mut writer);
        in_2.ser(&mut writer);
        in_3.ser(&mut writer);

        let buffer = writer.to_bytes();

        // Read
        let mut reader = BitReader::new(&buffer);

        let out_1 = Serde::de(&mut reader).unwrap();
        let out_2 = Serde::de(&mut reader).unwrap();
        let out_3 = Serde::de(&mut reader).unwrap();

        assert_eq!(in_1, out_1);
        assert_eq!(in_2, out_2);
        assert_eq!(in_3, out_3);
    }

    #[test]
    fn read_write_signed_variable() {
        // Write
        let mut writer = BitWriter::new();

        let in_1 = SignedVariableInteger::<5>::new(-668);
        let in_2 = SignedVariableInteger::<6>::new(53735);
        let in_3 = SignedVariableInteger::<2>::new(-3);

        in_1.ser(&mut writer);
        in_2.ser(&mut writer);
        in_3.ser(&mut writer);

        let buffer = writer.to_bytes();

        // Read
        let mut reader = BitReader::new(&buffer);

        let out_1 = Serde::de(&mut reader).unwrap();
        let out_2 = Serde::de(&mut reader).unwrap();
        let out_3 = Serde::de(&mut reader).unwrap();

        assert_eq!(in_1, out_1);
        assert_eq!(in_2, out_2);
        assert_eq!(in_3, out_3);
    }
}
