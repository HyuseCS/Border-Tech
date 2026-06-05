use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{info, debug, error};

/// Handles the WO Mic proprietary protocol.
/// Supports the initial handshake and robust audio packet extraction.
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

    /// Performs the WO Mic handshake (Init, Config, Start).
    pub async fn handshake(&mut self) -> anyhow::Result<()> {
        // Init: 'e' + length(6) + payload
        info!("Sending Init command");
        let init_payload = [0x65, 0x00, 0x00, 0x00, 0x06, 0x04, 0x04, 0x06, 0x02, 0x00, 0x00];
        self.stream.write_all(&init_payload).await?;

        let mut resp = [0u8; 7];
        self.stream.read_exact(&mut resp).await?;
        debug!("Init response: {:x?}", resp);

        // Config: 'f' + length(6) + payload
        info!("Sending Config command");
        // 0xBB, 0x80 = 48000 Hz sample rate
        let config_payload = [0x66, 0x00, 0x00, 0x00, 0x06, 0x02, 0x02, 0x00, 0x00, 0xBB, 0x80];
        self.stream.write_all(&config_payload).await?;

        let mut resp = [0u8; 6];
        self.stream.read_exact(&mut resp).await?;
        debug!("Config response: {:x?}", resp);

        // Start: 'g' + length(0)
        info!("Sending Start command");
        let start_payload = [0x67, 0x00, 0x00, 0x00, 0x00];
        self.stream.write_all(&start_payload).await?;

        let mut resp = [0u8; 6];
        self.stream.read_exact(&mut resp).await?;
        debug!("Start response: {:x?}", resp);

        Ok(())
    }

    /// Reads an audio packet from the stream.
    /// Implements a resynchronization mechanism to handle malformed or non-audio packets.
    pub async fn read_audio_packet(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> {
        // WO Mic audio packet format:
        //   Byte 0:    Packet type (0x04 = audio data)
        //   Byte 1-3:  Payload length as 24-bit big-endian integer
        //   Byte 4..N: Payload (raw PCM or Opus frames)
        //
        // SEC-03: Implement robust resync loop — scan byte-by-byte for 0x04 marker
        let mut header = [0u8; 4];
        
        loop {
            self.stream.read_exact(&mut header[0..1]).await?;
            if header[0] == 0x04 {
                // Found potential start of header — read the 3-byte length field
                self.stream.read_exact(&mut header[1..4]).await?;
                
                // Parse 24-bit big-endian length from header[1..4]
                let len = ((header[1] as usize) << 16)
                        | ((header[2] as usize) << 8)
                        |  (header[3] as usize);
                
                if len == 0 {
                    // Zero-length audio packet, skip
                    continue;
                }
                if len > buf.len() {
                    error!("Buffer overflow: packet length {} exceeds buffer size {}", len, buf.len());
                    return Err(anyhow::anyhow!(
                        "Buffer overflow: packet length {} exceeds buffer size {}", len, buf.len()
                    ));
                }
                
                self.stream.read_exact(&mut buf[..len]).await?;
                debug!("Read audio packet: len={}, header={:x?}", len, header);
                return Ok(len);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::io::Builder;

    #[tokio::test]
    async fn test_valid_packet_parsing() {
        let data = [
            0x04, 0x00, 0x00, 0x05, // Header
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
    async fn test_protocol_resync() {
        let data = [
            0xDE, 0xAD, 0xBE, 0xEF, // Garbage
            0x04, 0x00, 0x00, 0x03, // Header
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
            0x04, 0x00, 0x00, 0xFF, // Header claiming 255 bytes
        ];
        let mock = Builder::new().read(&data).build();
        let mut handler = ProtocolHandler::new(mock);
        let mut buf = [0u8; 10]; // Buffer too small

        let result = handler.read_audio_packet(&mut buf).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Buffer overflow"));
    }
}
