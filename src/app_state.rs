use slint::Weak;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::audio::PipewireSink;
use crate::decoder::{Decoder, PcmDecoder, OpusDecoder};
use crate::protocol::ProtocolHandler;
use crate::{MainWindow, Codec};
use tokio::net::TcpStream;
use tracing::{info, error};
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

/// Orchestrates the application state, managing connection tasks and UI synchronization.
pub struct AppState {
    ui: Weak<MainWindow>,
    tokio_runtime: tokio::runtime::Runtime,
    // REL-01: Store the actual join handle and cancel token
    connection_task: Arc<Mutex<Option<(tokio::task::JoinHandle<()>, tokio::sync::oneshot::Sender<()>)>>>,
}

impl AppState {
    /// Creates a new AppState.
    pub fn new(ui: Weak<MainWindow>) -> Self {
        Self {
            ui,
            tokio_runtime: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
            connection_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Initiates a connection to the mobile app.
    pub fn connect(&self, ip_str: String, port: u16, is_usb: bool, codec: Codec) {
        let ui_weak = self.ui.clone();
        let connection_task = self.connection_task.clone();

        // SEC-01: Validate IP address immediately
        let ip = match IpAddr::from_str(&ip_str) {
            Ok(addr) => addr,
            Err(_) => {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status_text(format!("Error: Invalid IP address '{}'", ip_str).into());
                    }
                });
                return;
            }
        };

        let ui_weak = self.ui.clone();
        self.tokio_runtime.spawn(async move {
            // REL-01: Cancel existing connection if any
            {
                let mut task_opt = connection_task.lock().await;
                if let Some((handle, tx)) = task_opt.take() {
                    info!("Cancelling existing connection...");
                    let _ = tx.send(());
                    let _ = handle.await;
                }

                let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
                let ui_weak_task = ui_weak.clone();
                let handle = tokio::spawn(async move {
                    // REL-03: Implementation of reconnection logic
                    let mut retry_count = 0;
                    const MAX_RETRIES: u32 = 5;

                    loop {
                        let res = Self::run_connection(ip, port, is_usb, codec, ui_weak_task.clone(), &mut rx).await;
                        
                        match res {
                            Ok(_) => break, // Clean exit (disconnect requested)
                            Err(e) => {
                                error!("Connection error: {}. Retry {}/{}", e, retry_count + 1, MAX_RETRIES);
                                
                                if rx.try_recv().is_ok() || retry_count >= MAX_RETRIES {
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(ui) = ui_weak_task.upgrade() {
                                            ui.set_is_connected(false);
                                            ui.set_status_text(format!("Error: {}", e).into());
                                        }
                                    });
                                    break;
                                }

                                retry_count += 1;
                                let _ = slint::invoke_from_event_loop({
                                    let ui_weak = ui_weak_task.clone();
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

    /// Disconnects from the current mobile app.
    pub fn disconnect(&self) {
        let connection_task = self.connection_task.clone();
        self.tokio_runtime.spawn(async move {
            let mut task_opt = connection_task.lock().await;
            if let Some((handle, tx)) = task_opt.take() {
                info!("Disconnecting...");
                let _ = tx.send(());
                let _ = handle.await;
            }
        });
    }

    async fn run_connection(
        ip: IpAddr, 
        port: u16, 
        is_usb: bool, 
        codec: Codec,
        ui_weak: Weak<MainWindow>,
        stop_rx: &mut tokio::sync::oneshot::Receiver<()>
    ) -> anyhow::Result<()> {
        let _ = slint::invoke_from_event_loop({
            let ui_weak = ui_weak.clone();
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_status_text("Connecting...".into());
                }
            }
        });

        // SEC-05: ADB Forward Cleanup using a scope guard
        let _adb_cleanup = if is_usb {
            info!("Setting up ADB port forwarding...");
            let output = tokio::process::Command::new("adb")
                .arg("forward")
                .arg(format!("tcp:{}", port))
                .arg(format!("tcp:{}", port))
                .output()
                .await?;
            
            if !output.status.success() {
                return Err(anyhow::anyhow!("ADB forward failed: {}", String::from_utf8_lossy(&output.stderr)));
            }

            Some(scopeguard::guard(port, |p| {
                info!("Removing ADB port forwarding for port {}...", p);
                let _ = std::process::Command::new("adb")
                    .arg("forward")
                    .arg("--remove")
                    .arg(format!("tcp:{}", p))
                    .output();
            }))
        } else {
            None
        };

        let stream = TcpStream::connect(format!("{}:{}", ip, port)).await?;
        let mut protocol = ProtocolHandler::new(stream);
        protocol.handshake().await?;
        
        let sink = PipewireSink::new("WOMIC-Virtual-Mic".to_string(), 48000)?;
        
        // ARCH-02: Use user-selected codec
        let mut decoder: Box<dyn Decoder> = match codec {
            Codec::Pcm => Box::new(PcmDecoder::new()),
            Codec::Opus => Box::new(OpusDecoder::new(48000, opus::Channels::Mono)?),
        };

        let _ = slint::invoke_from_event_loop({
            let ui_weak = ui_weak.clone();
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_is_connected(true);
                    ui.set_status_text("Connected".into());
                }
            }
        });

        let mut buf = [0u8; 4096];
        let mut float_buf = Vec::with_capacity(4096);

        loop {
            tokio::select! {
                // REL-01: Check if stop requested periodically
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    if stop_rx.try_recv().is_ok() {
                        info!("Disconnect requested");
                        return Ok(());
                    }
                }
                res = protocol.read_audio_packet(&mut buf) => {
                    match res {
                        Ok(0) => continue,
                        Ok(n) => {
                            float_buf.clear();
                            if let Err(e) = decoder.decode(&buf[..n], &mut float_buf) {
                                error!("Decoding error: {}", e);
                                continue;
                            }
                            
                            let mut max_abs = 0.0f32;
                            for &s in &float_buf {
                                max_abs = max_abs.max(s.abs());
                            }
                            
                            sink.push_samples(&float_buf);
                            
                            let _ = slint::invoke_from_event_loop({
                                let ui_weak = ui_weak.clone();
                                move || {
                                    if let Some(ui) = ui_weak.upgrade() {
                                        ui.set_volume_level(max_abs);
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Reception error: {}", e));
                        }
                    }
                }
            }
        }
    }
}
