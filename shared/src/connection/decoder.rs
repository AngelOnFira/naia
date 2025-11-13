cfg_if! {
    if #[cfg(feature = "zstd_support")]
    {
        use zstd::bulk::Decompressor;

        use super::compression_config::CompressionMode;
        use super::error::DecoderError;

        pub struct Decoder {
            result: Vec<u8>,
            decoder: Option<Decompressor<'static>>,
        }

        impl Decoder {
            /// Try to create a new Decoder with the specified compression mode
            pub fn try_new(compression_mode: CompressionMode) -> Result<Self, DecoderError> {
                let decoder = match compression_mode {
                    CompressionMode::Training(_) => None,
                    CompressionMode::Default(_) => {
                        Some(Decompressor::new().map_err(|_| DecoderError::DecompressorCreationFailed)?)
                    }
                    CompressionMode::Dictionary(_, dictionary) => Some(
                        Decompressor::with_dictionary(&dictionary).map_err(|_| DecoderError::DecompressorWithDictionaryFailed)?,
                    ),
                };

                Ok(Self {
                    decoder,
                    result: Vec::new(),
                })
            }

            /// Create a new Decoder with the specified compression mode
            ///
            /// # Panics
            /// Panics if the decompressor cannot be created with the given configuration
            pub fn new(compression_mode: CompressionMode) -> Self {
                Self::try_new(compression_mode).expect("Failed to create Decoder")
            }

            /// Try to decode a payload, returning error on decompression failure
            ///
            /// SECURITY: This method processes untrusted network data. Any malformed or
            /// malicious payload will return an error instead of panicking.
            pub fn try_decode(&mut self, payload: &[u8]) -> Result<&[u8], DecoderError> {
                if let Some(decoder) = &mut self.decoder {
                    let upper_bound = Decompressor::<'static>::upper_bound(payload)
                        .map_err(|_| DecoderError::UpperBoundCalculationFailed {
                            payload_size: payload.len(),
                        })?;

                    self.result = decoder
                        .decompress(payload, upper_bound)
                        .map_err(|_| DecoderError::DecompressionFailed {
                            payload_size: payload.len(),
                        })?;
                    Ok(&self.result)
                } else {
                    self.result = payload.to_vec();
                    Ok(&self.result)
                }
            }

            /// Decode a payload
            ///
            /// # Panics
            /// Panics if decompression fails
            pub fn decode(&mut self, payload: &[u8]) -> &[u8] {
                self.try_decode(payload).expect("Failed to decode payload")
            }
        }
    }
    else
    {
        use super::compression_config::CompressionMode;

        pub struct Decoder {
            result: Vec<u8>,
        }

        impl Decoder {
            pub fn new(_: CompressionMode) -> Self {
                Self {
                    result: Vec::new(),
                }
            }

            pub fn decode(&mut self, payload: &[u8]) -> &[u8] {
                self.result = payload.to_vec();
                &self.result
            }
        }
    }
}
