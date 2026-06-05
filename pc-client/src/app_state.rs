use slint::Weak;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::audio::PipewireSink;
use crate::protocol::ProtocolHandler;
use crate::MainWindow;
use tokio::net::TcpListener;
use tracing::{info, error};
use std::time::Duration;

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

    /// Starts listening for incoming audio connections.
    pub fn connect(&self, port: u16, is_usb: bool) {
        let connection_task = self.connection_task.clone();

        let ui_weak_task = self.ui.clone();
        tokio::spawn(async move {
            // Cancel existing listener if any
            {
                let mut task_opt = connection_task.lock().await;
                if let Some((handle, tx)) = task_opt.take() {
                    info!("Cancelling existing connection listener...");
                    let _ = tx.send(());
                    let _ = handle.await;
                }

                let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
                let ui_weak_run = ui_weak_task.clone();
                let handle = tokio::spawn(async move {
                    let mut retry_count = 0;
                    const MAX_RETRIES: u32 = 5;

                    loop {
                        let res = Self::run_connection(port, is_usb, ui_weak_run.clone(), &mut rx).await;
                        
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
        ui_weak: Weak<MainWindow>,
        stop_rx: &mut tokio::sync::oneshot::Receiver<()>
    ) -> anyhow::Result<()> {
        let _ = slint::invoke_from_event_loop({
            let ui_weak = ui_weak.clone();
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_status_text("Waiting for connection...".into());
                }
            }
        });

        // Set up ADB reverse forwarding if USB mode is active
        let _adb_cleanup = if is_usb {
            info!("Configuring ADB port reverse forwarding for port {}...", port);
            let out = tokio::process::Command::new("adb")
                .args(["reverse", &format!("tcp:{}", port), &format!("tcp:{}", port)])
                .output().await?;
            
            if !out.status.success() {
                return Err(anyhow::anyhow!("ADB reverse command failed: {}", String::from_utf8_lossy(&out.stderr)));
            }

            Some(scopeguard::guard(port, |p| {
                info!("Removing ADB port reverse forwarding...");
                let _ = std::process::Command::new("adb")
                    .args(["reverse", "--remove", &format!("tcp:{}", p)])
                    .output();
            }))
        } else {
            None
        };

        // Bind TCP Listener
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        info!("Listening for phone connection on port {}", port);

        let mut stop_fut = stop_rx;

        // Accept connection or handle cancellation
        let (stream, peer_addr) = tokio::select! {
            res = listener.accept() => res?,
            _ = &mut stop_fut => {
                info!("Listening stopped before connection was accepted.");
                return Ok(());
            }
        };

        info!("Accepted connection from {}", peer_addr);
        
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
    let sample_count = s16_bytes.len() / 2;
    for i in 0..sample_count {
        let low = s16_bytes[i * 2];
        let high = s16_bytes[i * 2 + 1];
        let sample = i16::from_le_bytes([low, high]);
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
