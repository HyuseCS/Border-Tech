/// Trait for audio decoders that convert raw payloads to PCM float buffers.
pub trait Decoder: Send {
    /// Decodes the input bytes into the output vector of f32 samples.
    fn decode(&mut self, input: &[u8], output: &mut Vec<f32>) -> anyhow::Result<()>;
}

/// Linear PCM decoder (S16LE to F32).
pub struct PcmDecoder;

impl PcmDecoder {
    /// Creates a new PCM decoder.
    pub fn new() -> Self {
        Self
    }
}

impl Decoder for PcmDecoder {
    fn decode(&mut self, input: &[u8], output: &mut Vec<f32>) -> anyhow::Result<()> {
        // S16LE to F32
        for i in (0..input.len()).step_by(2) {
            if i + 1 < input.len() {
                let sample = i16::from_le_bytes([input[i], input[i + 1]]);
                output.push(sample as f32 / 32768.0);
            }
        }
        Ok(())
    }
}

/// Opus decoder.
pub struct OpusDecoder {
    decoder: opus::Decoder,
}

impl OpusDecoder {
    /// Creates a new Opus decoder with the given sample rate and channels.
    pub fn new(sample_rate: u32, channels: opus::Channels) -> anyhow::Result<Self> {
        let decoder = opus::Decoder::new(sample_rate, channels)?;
        Ok(Self { decoder })
    }
}

impl Decoder for OpusDecoder {
    fn decode(&mut self, input: &[u8], output: &mut Vec<f32>) -> anyhow::Result<()> {
        let mut pcm = [0.0f32; 1920]; // 40ms at 48kHz
        let n = self.decoder.decode_float(input, &mut pcm, false)?;
        output.extend_from_slice(&pcm[..n]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcm_decoding() {
        let mut decoder = PcmDecoder::new();
        // 0, 32767 (max), -32768 (min)
        let input = [
            0x00, 0x00, 
            0xFF, 0x7F, 
            0x00, 0x80
        ];
        let mut output = Vec::new();
        decoder.decode(&input, &mut output).unwrap();

        assert_eq!(output.len(), 3);
        assert!((output[0] - 0.0).abs() < 1e-6);
        assert!((output[1] - 32767.0 / 32768.0).abs() < 1e-6);
        assert!((output[2] - (-1.0)).abs() < 1e-6);
    }
}
