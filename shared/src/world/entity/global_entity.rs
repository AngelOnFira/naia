use crate::{BigMapKey, EntityError};
use naia_serde::{BitReader, BitWrite, Serde, SerdeErr};

// GlobalEntity
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct GlobalEntity(u64);

impl BigMapKey for GlobalEntity {
    fn to_u64(&self) -> u64 {
        self.0
    }

    fn from_u64(value: u64) -> Self {
        GlobalEntity(value)
    }
}

impl Serde for GlobalEntity {
    /// GlobalEntity serialization is not supported - entities are serialized as LocalEntity variants.
    ///
    /// # Panics
    ///
    /// This method always panics as GlobalEntity should never be directly serialized.
    /// Use LocalEntity (HostEntity/RemoteEntity) for serialization instead.
    fn ser(&self, _: &mut dyn BitWrite) {
        panic!("GlobalEntity serialization not supported - use LocalEntity instead");
    }

    /// GlobalEntity deserialization is not supported - entities are deserialized as LocalEntity variants.
    ///
    /// # Panics
    ///
    /// This method always panics as GlobalEntity should never be directly deserialized.
    /// Use LocalEntity (HostEntity/RemoteEntity) for deserialization instead.
    fn de(_: &mut BitReader) -> Result<Self, SerdeErr> {
        panic!("GlobalEntity deserialization not supported - use LocalEntity instead");
    }

    /// GlobalEntity bit length calculation is not supported.
    ///
    /// # Panics
    ///
    /// This method always panics as GlobalEntity should never be directly serialized.
    fn bit_length(&self) -> u32 {
        panic!("GlobalEntity bit_length not supported - use LocalEntity instead");
    }
}
