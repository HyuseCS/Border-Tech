use slint::Weak;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::audio::PipewireSink;
use crate::protocol::ProtocolHandler;
use crate::MainWindow;

use tracing::{info, error};
use std::time::Duration;

pub trait AsyncStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> AsyncStream for T {}

use std::fs;
use std::path::PathBuf;
use sha2::{Sha256, Digest};

#[derive(Debug)]
struct TofuVerifier {
    pin_path: PathBuf,
}

impl TofuVerifier {
    fn new() -> Self {
        let mut path = dirs::config_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
        path.push("project-m");
        fs::create_dir_all(&path).ok();
        path.push("pinned_cert.sha256");
        Self { pin_path: path }
    }
}

impl rustls::client::danger::ServerCertVerifier for TofuVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        let mut hasher = Sha256::new();
        hasher.update(end_entity.as_ref());
        let hash = hasher.finalize();
        let hash_hex = hex::encode(hash);

        if let Ok(pinned) = fs::read_to_string(&self.pin_path) {
            if pinned.trim() == hash_hex {
                Ok(rustls::client::danger::ServerCertVerified::assertion())
            } else {
                Err(rustls::Error::General(format!("Certificate pinning failed! Expected {}, got {}", pinned.trim(), hash_hex)))
            }
        } else {
            // No pin exists yet, accept for now (TOFU). We will save the pin after SPAKE2 succeeds.
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        }
    }
    
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

/// Orchestrates the application state, managing the TCP listener and UI synchronization.
pub struct AppState {
    ui: Weak<MainWindow>,
    connection_task: Arc<Mutex<Option<(tokio::task::JoinHandle<()>, tokio::sync::oneshot::Sender<()>)>>>,
}

impl AppState {
    /// Creates a new AppState.
    pub fn new(ui: Weak<MainWindow>) -> Self {
        Self {
            ui,
            connection_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Starts connection logic.
    pub fn connect(&self, port: u16, is_usb: bool, server_ip: String, auth_pin: String) {
        let connection_task = self.connection_task.clone();

        let ui_weak_task = self.ui.clone();
        tokio::spawn(async move {
            // Cancel existing listener/connection if any
            {
                let mut task_opt = connection_task.lock().await;
                if let Some((handle, tx)) = task_opt.take() {
                    info!("Cancelling existing connection...");
                    let _ = tx.send(());
                    let _ = handle.await;
                }

                let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
                let ui_weak_run = ui_weak_task.clone();
                let server_ip_clone = server_ip.clone();
                let handle = tokio::spawn(async move {
                    let mut retry_count = 0;
                    const MAX_RETRIES: u32 = 5;

                    loop {
                        let res = Self::run_connection(port, is_usb, server_ip_clone.clone(), auth_pin.clone(), ui_weak_run.clone(), &mut rx).await;
                        
                        match res {
                            Ok(_) => {
                                // Clean exit (disconnect requested)
                                let _ = slint::invoke_from_event_loop({
                                    let ui_weak = ui_weak_run.clone();
                                    move || {
                                        if let Some(ui) = ui_weak.upgrade() {
                                            ui.set_is_connected(false);
                                            ui.set_status_text("Disconnected".into());
                                            ui.set_volume_level(0.0);
                                        }
                                    }
                                });
                                break;
                            }
                            Err(e) => {
                                error!("Connection error: {}. Retry {}/{}", e, retry_count + 1, MAX_RETRIES);
                                
                                if rx.try_recv().is_ok() || retry_count >= MAX_RETRIES {
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(ui) = ui_weak_run.upgrade() {
                                            ui.set_is_connected(false);
                                            ui.set_status_text(format!("Error: {}", e).into());
                                        }
                                    });
                                    break;
                                }

                                retry_count += 1;
                                let _ = slint::invoke_from_event_loop({
                                    let ui_weak = ui_weak_run.clone();
                                    move || {
                                        if let Some(ui) = ui_weak.upgrade() {
                                            ui.set_status_text(format!("Retrying ({})...", retry_count).into());
                                        }
                                    }
                                });
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            }
                        }
                    }
                });
                *task_opt = Some((handle, tx));
            }
        });
    }

    /// Stops listening and disconnects the active stream.
    pub fn disconnect(&self) {
        let connection_task = self.connection_task.clone();
        tokio::spawn(async move {
            let mut task_opt = connection_task.lock().await;
            if let Some((handle, tx)) = task_opt.take() {
                info!("Disconnecting listener...");
                let _ = tx.send(());
                let _ = handle.await;
            }
        });
    }

    async fn run_connection(
        port: u16, 
        is_usb: bool, 
        server_ip: String,
        auth_pin: String,
        ui_weak: Weak<MainWindow>,
        stop_rx: &mut tokio::sync::oneshot::Receiver<()>
    ) -> anyhow::Result<()> {
        let status_msg = if is_usb {
            "Waiting for connection...".to_string()
        } else {
            format!("Connecting to {}...", server_ip)
        };

        let _ = slint::invoke_from_event_loop({
            let ui_weak = ui_weak.clone();
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_status_text(status_msg.into());
                }
            }
        });

        // Set up ADB reverse forwarding if USB mode is active
        let _adb_cleanup = if is_usb {
            let adb_path = std::process::Command::new("which")
                .arg("adb")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "/usr/bin/adb".to_string());

            info!("Configuring ADB port forwarding (PC -> Phone) for port {} using {}...", port, adb_path);
            
            // Proactively remove any existing dangling bindings to avoid "Address already in use" errors
            let _ = tokio::process::Command::new(&adb_path)
                .args(["forward", "--remove", &format!("tcp:{}", port)])
                .output().await;

            let out = tokio::process::Command::new(&adb_path)
                .args(["forward", &format!("tcp:{}", port), &format!("tcp:{}", port)])
                .output().await?;
            
            if !out.status.success() {
                return Err(anyhow::anyhow!("ADB forward command failed: {}", String::from_utf8_lossy(&out.stderr)));
            }

            Some(scopeguard::guard((port, adb_path), |(p, adb_cmd)| {
                info!("Removing ADB port forwarding...");
                let _ = std::process::Command::new(&adb_cmd)
                    .args(["forward", "--remove", &format!("tcp:{}", p)])
                    .output();
            }))
        } else {
            None
        };

        let mut stop_fut = stop_rx;

        let target_addr = if is_usb {
            format!("127.0.0.1:{}", port)
        } else {
            format!("{}:{}", server_ip, port)
        };

        info!("Connecting to Android device at {}...", target_addr);

        let tcp_stream = tokio::select! {
            res = tokio::net::TcpStream::connect(&target_addr) => res?,
            _ = &mut stop_fut => {
                info!("Connection attempt cancelled.");
                return Ok(());
            }
        };
        let peer_addr = tcp_stream.peer_addr()?;
        tcp_stream.set_nodelay(true)?;
        
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(TofuVerifier::new()))
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        let ip_str = if is_usb { "127.0.0.1" } else { server_ip.as_str() };
        let domain = rustls::pki_types::ServerName::try_from(ip_str).unwrap_or_else(|_| {
            rustls::pki_types::ServerName::try_from("localhost").unwrap()
        }).to_owned();
        
        info!("Initiating TLS handshake...");
        let tls_stream = tokio::select! {
            res = connector.connect(domain, tcp_stream) => res?,
            _ = &mut stop_fut => {
                info!("Connection cancelled during TLS handshake.");
                return Ok(());
            }
        };

        // Save TOFU pin if not already pinned
        if let Some(certs) = tls_stream.get_ref().1.peer_certificates() {
            if let Some(end_entity) = certs.first() {
                let mut hasher = Sha256::new();
                hasher.update(end_entity.as_ref());
                let hash = hasher.finalize();
                let hash_hex = hex::encode(hash);

                let mut path = dirs::config_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
                path.push("project-m");
                let pin_path = path.join("pinned_cert.sha256");
                if !pin_path.exists() {
                    info!("Pinning server certificate: {}", hash_hex);
                    let _ = std::fs::write(pin_path, hash_hex);
                }
            }
        }

        let mut stream: Box<dyn AsyncStream> = Box::new(tls_stream);

        info!("Sending authentication PIN via SRP...");
        use sha2::Sha256;
        use srp::{ClientG2048, EphemeralSecret, Generate};
        let srp_client = ClientG2048::<Sha256>::new();
        let a_sec = EphemeralSecret::generate();
        let a_pub = srp_client.compute_public_ephemeral(&a_sec);
        let a_bytes = &a_pub;

        info!("SRP: A computed (len={})", a_bytes.len());

        let mut auth_pkt = vec![b'S', b'R', b'P', b'1'];
        // Ensure A is 256 bytes, pad if needed
        let mut a_pad = vec![0u8; 256];
        let offset = 256usize.saturating_sub(a_bytes.len());
        let len = std::cmp::min(256, a_bytes.len());
        a_pad[offset..offset+len].copy_from_slice(&a_bytes[a_bytes.len()-len..]);
        
        auth_pkt.extend_from_slice(&a_pad);
        stream.write_all(&auth_pkt).await?;
        stream.flush().await?;
        info!("SRP: Sent SRP1 (A)");

        // Read SRP2
        let mut srp2_hdr = [0u8; 4];
        stream.read_exact(&mut srp2_hdr).await?;
        if &srp2_hdr != b"SRP2" {
            error!("SRP: Invalid SRP2 header: {:?}", srp2_hdr);
            return Err(anyhow::anyhow!("Invalid SRP2 header"));
        }
        let mut salt = [0u8; 16];
        stream.read_exact(&mut salt).await?;
        let mut b_bytes = [0u8; 256];
        stream.read_exact(&mut b_bytes).await?;
        info!("SRP: Received SRP2 (salt + B)");

        // Process reply
        let verifier = srp_client.process_reply(&a_sec, b"client", auth_pin.as_bytes(), &salt, &b_bytes)
            .map_err(|e| {
                error!("SRP: process_reply failed: {:?}", e);
                anyhow::anyhow!("SRP invalid server B")
            })?;
        
        let s_bytes = verifier.key();
        let mut s_pad = vec![0u8; 256];
        let offset = 256usize.saturating_sub(s_bytes.len());
        let len = std::cmp::min(256, s_bytes.len());
        s_pad[offset..offset+len].copy_from_slice(&s_bytes[s_bytes.len()-len..]);

        let mut hasher = Sha256::new();
        hasher.update(b"M1");
        hasher.update(&s_pad);
        let custom_m1 = hasher.finalize();

        // Send SRP3
        let mut srp3_pkt = vec![b'S', b'R', b'P', b'3'];
        srp3_pkt.extend_from_slice(&custom_m1);
        stream.write_all(&srp3_pkt).await?;
        stream.flush().await?;
        info!("SRP: Sent SRP3 (Custom M1)");

        // Read SRP4
        let mut srp4_hdr = [0u8; 4];
        stream.read_exact(&mut srp4_hdr).await?;
        if &srp4_hdr != b"SRP4" {
            error!("SRP: Invalid SRP4 header: {:?}", srp4_hdr);
            return Err(anyhow::anyhow!("Invalid SRP4 header"));
        }
        let mut m2 = [0u8; 32];
        stream.read_exact(&mut m2).await?;
        info!("SRP: Received SRP4 (Custom M2)");

        let mut hasher2 = Sha256::new();
        hasher2.update(b"M2");
        hasher2.update(&s_pad);
        let expected_m2 = hasher2.finalize();

        use subtle::ConstantTimeEq;
        if expected_m2.ct_eq(&m2).unwrap_u8() != 1 {
            return Err(anyhow::anyhow!("SRP server verification failed"));
        }

        info!("SRP: Handshake successful!");


        info!("Connection established with {}", peer_addr);
        
        let _ = slint::invoke_from_event_loop({
            let ui_weak = ui_weak.clone();
            let peer = peer_addr.to_string();
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_is_connected(true);
                    ui.set_status_text(format!("Connected to {}", peer).into());
                }
            }
        });

        // Initialize PipeWire virtual source sink at 48kHz
        let sink = PipewireSink::new("Lampyris-Virtual-Mic".to_string(), 48000)?;
        let mut protocol = ProtocolHandler::new(stream);
        
        // 960 bytes = 480 samples of 16-bit Mono @ 48kHz
        let mut buf = [0u8; 960];
        let mut float_buf = Vec::with_capacity(480);

        loop {
            tokio::select! {
                biased;

                _ = &mut stop_fut => {
                    info!("Disconnect requested, exiting stream loop.");
                    return Ok(());
                }

                res = protocol.read_audio_packet(&mut buf) => {
                    match res {
                        Ok(0) => continue,
                        Ok(n) => {
                            // Convert s16le bytes to f32 samples
                            convert_s16_to_f32(&buf[..n], &mut float_buf);

                            // Calculate peak amplitude for the UI progress indicator
                            let peak_vol = calculate_peak_amplitude(&float_buf);

                            // Pipe raw samples into PipeWire
                            sink.push_samples(&float_buf);

                            // Push peak volume level to Slint UI
                            let _ = slint::invoke_from_event_loop({
                                let ui_weak = ui_weak.clone();
                                move || {
                                    if let Some(ui) = ui_weak.upgrade() {
                                        ui.set_volume_level(peak_vol);
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Stream read error: {}", e));
                        }
                    }
                }
            }
        }
    }
}

/// Converts little-endian 16-bit PCM bytes into normalized f32 audio samples.
fn convert_s16_to_f32(s16_bytes: &[u8], dst: &mut Vec<f32>) {
    dst.clear();
    for chunk in s16_bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        dst.push(sample as f32 / 32768.0);
    }
}

/// Computes the peak amplitude (max absolute value) in a slice of audio samples.
fn calculate_peak_amplitude(samples: &[f32]) -> f32 {
    let mut max = 0.0f32;
    for &s in samples {
        max = max.max(s.abs());
    }
    max
}
