cfg_if! {
    if #[cfg(feature = "zstd_support")]
    {
        use std::fs;

        use log::info;

        use zstd::{bulk::Compressor, dict::from_continuous};

        use super::compression_config::CompressionMode;
        use super::error::EncoderError;

        pub struct Encoder {
            result: Vec<u8>,
            encoder: EncoderType,
        }

        impl Encoder {
            /// Try to create a new Encoder with the specified compression mode
            pub fn try_new(compression_mode: CompressionMode) -> Result<Self, EncoderError> {
                let encoder = match compression_mode {
                    CompressionMode::Training(sample_size) => {
                        EncoderType::DictionaryTrainer(DictionaryTrainer::new(sample_size))
                    }
                    CompressionMode::Default(compression_level) => EncoderType::Compressor(
                        Compressor::new(compression_level).map_err(|_| EncoderError::CompressorCreationFailed {
                            level: compression_level,
                        })?,
                    ),
                    CompressionMode::Dictionary(compression_level, dictionary) => EncoderType::Compressor(
                        Compressor::with_dictionary(compression_level, &dictionary)
                            .map_err(|_| EncoderError::CompressorWithDictionaryFailed {
                                level: compression_level,
                            })?,
                    ),
                };

                Ok(Self {
                    result: Vec::new(),
                    encoder,
                })
            }

            /// Create a new Encoder with the specified compression mode
            ///
            /// # Panics
            /// Panics if the compressor cannot be created with the given configuration
            pub fn new(compression_mode: CompressionMode) -> Self {
                Self::try_new(compression_mode).expect("Failed to create Encoder")
            }

            /// Try to encode a payload, returning error on compression failure
            pub fn try_encode(&mut self, payload: &[u8]) -> Result<&[u8], EncoderError> {
                // TODO: only use compressed packet if the resulting size would be less!
                match &mut self.encoder {
                    EncoderType::DictionaryTrainer(trainer) => {
                        trainer.record_bytes(payload);
                        self.result = payload.to_vec();
                        Ok(&self.result)
                    }
                    EncoderType::Compressor(encoder) => {
                        self.result = encoder.compress(payload).map_err(|_| EncoderError::CompressionFailed {
                            payload_size: payload.len(),
                        })?;
                        Ok(&self.result)
                    }
                }
            }

            /// Encode a payload
            ///
            /// # Panics
            /// Panics if compression fails
            pub fn encode(&mut self, payload: &[u8]) -> &[u8] {
                self.try_encode(payload).expect("Failed to encode payload")
            }
        }

        pub enum EncoderType {
            Compressor(Compressor<'static>),
            DictionaryTrainer(DictionaryTrainer),
        }

        pub struct DictionaryTrainer {
            sample_data: Vec<u8>,
            sample_sizes: Vec<usize>,
            next_alert_size: usize,
            target_sample_size: usize,
            training_complete: bool,
        }

        impl DictionaryTrainer {
            /// `target_sample_size` here describes the number of samples (packets) to
            /// train on. Obviously, the more samples trained on, the better
            /// theoretical compression.
            pub fn new(target_sample_size: usize) -> Self {
                Self {
                    target_sample_size,
                    sample_data: Vec::new(),
                    sample_sizes: Vec::new(),
                    next_alert_size: 0,
                    training_complete: false,
                }
            }

            /// Try to record bytes for dictionary training, returning error on failure
            pub fn try_record_bytes(&mut self, bytes: &[u8]) -> Result<(), EncoderError> {
                if self.training_complete {
                    return Ok(());
                }

                self.sample_data.extend_from_slice(bytes);
                self.sample_sizes.push(bytes.len());

                let current_sample_size = self.sample_sizes.len();

                if current_sample_size >= self.next_alert_size {
                    let percent =
                        ((self.next_alert_size as f32) / (self.target_sample_size as f32)) * 100.0;
                    info!("Dictionary training: {}% complete", percent);

                    self.next_alert_size += self.target_sample_size / 20;
                }

                if current_sample_size >= self.target_sample_size {
                    info!("Dictionary training complete!");
                    info!(
                        "Samples: {} ({} KB)",
                        self.sample_sizes.len(),
                        self.sample_data.len()
                    );
                    info!("Dictionary processing sample data...");

                    // We have enough sample data to train the dictionary!
                    let target_dict_size = self.sample_data.len() / 100;
                    let dictionary =
                        from_continuous(&self.sample_data, &self.sample_sizes, target_dict_size)
                            .map_err(|_| EncoderError::DictionaryTrainingFailed {
                                sample_count: self.sample_sizes.len(),
                                total_bytes: self.sample_data.len(),
                            })?;

                    // Now need to ... write it to a file I guess
                    fs::write("dictionary.txt", &dictionary)
                        .map_err(|_| EncoderError::DictionaryWriteFailed {
                            path: "dictionary.txt",
                        })?;

                    info!("Dictionary written to `dictionary.txt`!");

                    self.training_complete = true;
                }

                Ok(())
            }

            /// Record bytes for dictionary training
            ///
            /// # Panics
            /// Panics if dictionary training or writing fails
            pub fn record_bytes(&mut self, bytes: &[u8]) {
                self.try_record_bytes(bytes).expect("Failed to record bytes for dictionary training")
            }
        }
    }
    else
    {
        use super::compression_config::CompressionMode;

        pub struct Encoder {
            result: Vec<u8>
        }

        impl Encoder {
            pub fn new(_: CompressionMode) -> Self {
                Self {
                    result: Vec::new(),
                }
            }

            pub fn encode(&mut self, payload: &[u8]) -> &[u8] {
                self.result = payload.to_vec();
                &self.result
            }
        }
    }
}
