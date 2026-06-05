use slint::Weak;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::audio::PipewireSink;
use crate::protocol::ProtocolHandler;
use crate::MainWindow;
use tokio::net::TcpListener;
use tracing::{info, error};
use std::time::Duration;

pub trait AsyncStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> AsyncStream for T {}

#[derive(Debug)]
struct DummyVerifier;
impl rustls::client::danger::ServerCertVerifier for DummyVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
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
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
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
    pub fn connect(&self, port: u16, is_usb: bool, server_ip: String, use_tls: bool) {
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
                        let res = Self::run_connection(port, is_usb, server_ip_clone.clone(), use_tls, ui_weak_run.clone(), &mut rx).await;
                        
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
        use_tls: bool,
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

            info!("Configuring ADB port reverse forwarding for port {} using {}...", port, adb_path);
            let out = tokio::process::Command::new(&adb_path)
                .args(["reverse", &format!("tcp:{}", port), &format!("tcp:{}", port)])
                .output().await?;
            
            if !out.status.success() {
                return Err(anyhow::anyhow!("ADB reverse command failed: {}", String::from_utf8_lossy(&out.stderr)));
            }

            Some(scopeguard::guard((port, adb_path), |(p, adb_cmd)| {
                info!("Removing ADB port reverse forwarding...");
                let _ = std::process::Command::new(&adb_cmd)
                    .args(["reverse", "--remove", &format!("tcp:{}", p)])
                    .output();
            }))
        } else {
            None
        };

        let mut stop_fut = stop_rx;

        let (mut stream, peer_addr): (Box<dyn AsyncStream>, std::net::SocketAddr) = if is_usb {
            // Bind TCP Listener
            let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
            info!("Listening for phone connection on loopback port {}", port);

            // Accept connection or handle cancellation
            let (tcp_stream, peer_addr) = tokio::select! {
                res = listener.accept() => res?,
                _ = &mut stop_fut => {
                    info!("Listening stopped before connection was accepted.");
                    return Ok(());
                }
            };
            
            if use_tls {
                let cert = rcgen::generate_simple_self_signed(vec!["project-m.local".to_string()])?;
                let key_der = cert.signing_key.serialize_der();
                let cert_der = cert.cert.der().to_vec();
                let key = rustls::pki_types::PrivateKeyDer::try_from(key_der)
                    .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?;
                let certs = vec![rustls::pki_types::CertificateDer::from(cert_der)];

                let config = rustls::ServerConfig::builder()
                    .with_no_client_auth()
                    .with_single_cert(certs, key)?;

                let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(config));
                
                info!("Accepting TLS connection...");
                let tls_stream = tokio::select! {
                    res = acceptor.accept(tcp_stream) => res?,
                    _ = &mut stop_fut => {
                        info!("Listening stopped during TLS handshake.");
                        return Ok(());
                    }
                };
                (Box::new(tls_stream), peer_addr)
            } else {
                (Box::new(tcp_stream), peer_addr)
            }
        } else {
            // Wi-Fi Mode: Connect to Android device as a client
            let target_addr = format!("{}:{}", server_ip, port);
            info!("Connecting to Android device at {}...", target_addr);

            let tcp_stream = tokio::select! {
                res = tokio::net::TcpStream::connect(&target_addr) => res?,
                _ = &mut stop_fut => {
                    info!("Connection attempt cancelled.");
                    return Ok(());
                }
            };
            let peer_addr = tcp_stream.peer_addr()?;
            
            if use_tls {
                let mut config = rustls::ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(std::sync::Arc::new(DummyVerifier))
                    .with_no_client_auth();
                let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
                let domain = rustls::pki_types::ServerName::try_from("project-m.local").unwrap();
                
                info!("Initiating TLS handshake...");
                let tls_stream = tokio::select! {
                    res = connector.connect(domain, tcp_stream) => res?,
                    _ = &mut stop_fut => {
                        info!("Connection cancelled during TLS handshake.");
                        return Ok(());
                    }
                };
                (Box::new(tls_stream), peer_addr)
            } else {
                (Box::new(tcp_stream), peer_addr)
            }
        };

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
        let sink = PipewireSink::new("Project-M-Virtual-Mic".to_string(), 48000)?;
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
