use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tracing::{debug, error, warn};

/// Handles the Lampyris raw PCM framing protocol (v1).
/// Packets are structured as:
/// `['M', 'C', version_flags (1B), seq_num (2B BE), payload_size (2B BE)]` + `[raw PCM payload]`.
pub struct ProtocolHandler<S> {
    stream: S,
    expected_seq: Option<u16>,
    rx_queue: Vec<u8>,
}

impl<S> ProtocolHandler<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    /// Creates a new ProtocolHandler wrapping an asynchronous stream.
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            expected_seq: None,
            rx_queue: Vec::new(),
        }
    }

    /// Reads the next frame from the network stream, parses its header, and returns the payload, sequence number, and is_24khz flag.
    async fn read_next_frame(&mut self) -> anyhow::Result<(Vec<u8>, u16, bool)> {
        let mut header = [0u8; 7];

        loop {
            // Scan for magic 'M'
            self.stream.read_exact(&mut header[0..1]).await?;
            if header[0] == b'M' {
                // Check if next byte is 'C'
                self.stream.read_exact(&mut header[1..2]).await?;
                if header[1] == b'C' {
                    // Read the remaining 5 bytes of the header:
                    // Ver/Flags (1B), Seq No (2B BE), Payload Size (2B BE)
                    self.stream.read_exact(&mut header[2..7]).await?;

                    let ver_flags = header[2];
                    let version = ver_flags >> 4;
                    let flags = ver_flags & 0x0F;
                    let is_24khz = (flags & 0x01) != 0;

                    if version != 1 {
                        error!("Unsupported protocol version: {}", version);
                        return Err(anyhow::anyhow!("Unsupported protocol version: {}", version));
                    }

                    let seq = u16::from_be_bytes([header[3], header[4]]);
                    let len = u16::from_be_bytes([header[5], header[6]]) as usize;

                    if len == 0 {
                        continue;
                    }
                    if len > 4800 {
                        error!("Payload size {} exceeds limit of 4800 bytes", len);
                        return Err(anyhow::anyhow!(
                            "Payload size {} exceeds limit of 4800 bytes",
                            len
                        ));
                    }

                    let mut payload = vec![0u8; len];
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        self.stream.read_exact(&mut payload),
                    )
                    .await
                    {
                        Ok(res) => {
                            res?;
                        }
                        Err(_) => {
                            error!("Timeout waiting for audio packet payload ({} bytes)", len);
                            return Err(anyhow::anyhow!(
                                "Timeout waiting for audio packet payload"
                            ));
                        }
                    }
                    debug!(
                        "Read Lampyris audio packet: seq={}, len={}, 24kHz={}",
                        seq, len, is_24khz
                    );
                    return Ok((payload, seq, is_24khz));
                }
            }
        }
    }

    /// Reads an audio packet from the stream.
    /// Handles sequence gap detection (inserting silence) and upsampling internally.
    pub async fn read_audio_packet(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> {
        while self.rx_queue.len() < buf.len() {
            let (payload, seq, is_24khz) = self.read_next_frame().await?;

            // Sequence gap detection
            if let Some(expected) = self.expected_seq {
                let gap = seq.wrapping_sub(expected);
                if gap > 0 && gap < 3000 {
                    // Calculate silence bytes to insert: each missing frame is assumed to be
                    // the same size as the current frame (upsampled to 48kHz if 24kHz).
                    let frame_size = if is_24khz {
                        payload.len() * 2
                    } else {
                        payload.len()
                    };
                    let silence_len = (gap as usize * frame_size).min(48000); // Cap silence at 0.5s

                    warn!(
                        "Sequence gap detected: expected {}, got {}. Inserting {} bytes of silence.",
                        expected, seq, silence_len
                    );

                    self.rx_queue.resize(self.rx_queue.len() + silence_len, 0);
                }
            }
            self.expected_seq = Some(seq.wrapping_add(1));

            // Process payload: upsample if degraded (24kHz)
            if is_24khz {
                if payload.len() % 2 != 0 {
                    error!("Degraded payload length {} is not even", payload.len());
                    return Err(anyhow::anyhow!("Degraded payload length is not even"));
                }
                let mut upsampled = Vec::with_capacity(payload.len() * 2);
                for chunk in payload.chunks_exact(2) {
                    // Duplicate 16-bit sample (2 bytes)
                    upsampled.extend_from_slice(chunk);
                    upsampled.extend_from_slice(chunk);
                }
                self.rx_queue.extend_from_slice(&upsampled);
            } else {
                self.rx_queue.extend_from_slice(&payload);
            }
        }

        let to_read = buf.len().min(self.rx_queue.len());
        buf[..to_read].copy_from_slice(&self.rx_queue[..to_read]);
        self.rx_queue.drain(..to_read);
        Ok(to_read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn test_valid_mc_packet_parsing() {
        let data = [
            b'M', b'C', 0x10, 0x00, 0x01, 0x00, 0x05, // Header: v1/48kHz, seq 1, len 5
            0x01, 0x02, 0x03, 0x04, 0x05, // Payload
        ];
        let mock = Builder::new().read(&data).build();
        let mut handler = ProtocolHandler::new(mock);
        let mut buf = [0u8; 5];

        let n = handler.read_audio_packet(&mut buf).await.unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..5], &[0x01, 0x02, 0x03, 0x04, 0x05]);
    }

    #[tokio::test]
    async fn test_protocol_resync_mc() {
        let data = [
            0xDE, 0xAD, b'M', 0xBE, // Garbage containing 'M' but not 'C'
            b'M', b'C', 0x10, 0x00, 0x01, 0x00, 0x03, // Correct header
            0xAA, 0xBB, 0xCC, // Payload
        ];
        let mock = Builder::new().read(&data).build();
        let mut handler = ProtocolHandler::new(mock);
        let mut buf = [0u8; 3];

        let n = handler.read_audio_packet(&mut buf).await.unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[0xAA, 0xBB, 0xCC]);
    }

    #[tokio::test]
    async fn test_buffer_overflow_protection() {
        let data = [
            b'M', b'C', 0x10, 0x00, 0x01, 0x12, 0xC1, // Header claiming 4801 bytes (0x12C1)
        ];
        let mock = Builder::new().read(&data).build();
        let mut handler = ProtocolHandler::new(mock);
        let mut buf = [0u8; 10];

        let result = handler.read_audio_packet(&mut buf).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("exceeds limit of 4800")
        );
    }

    #[tokio::test]
    async fn test_degraded_mode_upsampling() {
        let data = [
            b'M', b'C', 0x11, 0x00, 0x01, 0x00, 0x04, // Header: v1/24kHz, seq 1, len 4
            0x01, 0x02, 0x03, 0x04, // Payload (2 samples: 0x0201, 0x0403)
        ];
        let mock = Builder::new().read(&data).build();
        let mut handler = ProtocolHandler::new(mock);
        let mut buf = [0u8; 8];

        let n = handler.read_audio_packet(&mut buf).await.unwrap();
        assert_eq!(n, 8);
        assert_eq!(&buf, &[0x01, 0x02, 0x01, 0x02, 0x03, 0x04, 0x03, 0x04]);
    }

    #[tokio::test]
    async fn test_sequence_gap_silence_insertion() {
        let data = [
            b'M', b'C', 0x10, 0x00, 0x01, 0x00, 0x04, // Header: seq 1, len 4
            0x01, 0x02, 0x03, 0x04, b'M', b'C', 0x10, 0x00, 0x03, 0x00,
            0x04, // Header: seq 3 (missing 2), len 4
            0x05, 0x06, 0x07, 0x08,
        ];
        let mock = Builder::new().read(&data).build();
        let mut handler = ProtocolHandler::new(mock);
        let mut buf = [0u8; 12];

        let n = handler.read_audio_packet(&mut buf).await.unwrap();
        assert_eq!(n, 12);
        assert_eq!(
            &buf,
            &[
                0x01, 0x02, 0x03, 0x04, // Packet 1
                0x00, 0x00, 0x00, 0x00, // Silence inserted for missing Packet 2
                0x05, 0x06, 0x07, 0x08 // Packet 3
            ]
        );
    }
}
