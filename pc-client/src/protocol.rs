use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tracing::{debug, error};

/// Handles the Lampyris raw PCM framing protocol.
/// Packets are structured as: `['M', 'C', length_high, length_low]` + `[raw PCM payload]`.
pub struct ProtocolHandler<S> {
    stream: S,
}

impl<S> ProtocolHandler<S> 
where 
    S: AsyncRead + AsyncWrite + Unpin + Send 
{
    /// Creates a new ProtocolHandler wrapping an asynchronous stream.
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    /// Reads an audio packet from the stream.
    /// Resynchronizes automatically if it encounters unexpected data by scanning for the 'M' 'C' marker.
    pub async fn read_audio_packet(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> {
        let mut header = [0u8; 4];
        
        loop {
            // Scan for magic 'M'
            self.stream.read_exact(&mut header[0..1]).await?;
            if header[0] == b'M' {
                // Check if next byte is 'C'
                self.stream.read_exact(&mut header[1..2]).await?;
                if header[1] == b'C' {
                    // Read payload length (2 bytes, big-endian)
                    self.stream.read_exact(&mut header[2..4]).await?;
                    let len = ((header[2] as usize) << 8) | (header[3] as usize);
                    
                    if len == 0 {
                        continue;
                    }
                    if len > buf.len() {
                        error!("Buffer overflow: packet length {} exceeds buffer size {}", len, buf.len());
                        return Err(anyhow::anyhow!(
                            "Buffer overflow: packet length {} exceeds buffer size {}", len, buf.len()
                        ));
                    }
                    
                    match tokio::time::timeout(std::time::Duration::from_secs(2), self.stream.read_exact(&mut buf[..len])).await {
                        Ok(res) => { res?; },
                        Err(_) => {
                            error!("Timeout waiting for audio packet payload ({} bytes)", len);
                            return Err(anyhow::anyhow!("Timeout waiting for audio packet payload"));
                        }
                    }
                    debug!("Read Lampyris audio packet: len={}", len);
                    return Ok(len);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn test_valid_mc_packet_parsing() {
        let data = [
            b'M', b'C', 0x00, 0x05, // Header claiming 5 bytes
            0x01, 0x02, 0x03, 0x04, 0x05, // Payload
        ];
        let mock = Builder::new().read(&data).build();
        let mut handler = ProtocolHandler::new(mock);
        let mut buf = [0u8; 10];

        let n = handler.read_audio_packet(&mut buf).await.unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..5], &[0x01, 0x02, 0x03, 0x04, 0x05]);
    }

    #[tokio::test]
    async fn test_protocol_resync_mc() {
        let data = [
            0xDE, 0xAD, b'M', 0xBE, // Garbage containing 'M' but not 'C'
            b'M', b'C', 0x00, 0x03, // Correct header
            0xAA, 0xBB, 0xCC, // Payload
        ];
        let mock = Builder::new().read(&data).build();
        let mut handler = ProtocolHandler::new(mock);
        let mut buf = [0u8; 10];

        let n = handler.read_audio_packet(&mut buf).await.unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], &[0xAA, 0xBB, 0xCC]);
    }

    #[tokio::test]
    async fn test_buffer_overflow_protection() {
        let data = [
            b'M', b'C', 0x01, 0x00, // Header claiming 256 bytes
        ];
        let mock = Builder::new().read(&data).build();
        let mut handler = ProtocolHandler::new(mock);
        let mut buf = [0u8; 10]; // Buffer is only 10 bytes

        let result = handler.read_audio_packet(&mut buf).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Buffer overflow"));
    }
}
