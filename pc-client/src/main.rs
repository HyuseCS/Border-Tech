mod audio;
mod protocol;
mod decoder;
mod app_state;

use clap::{Parser, ValueEnum};
use tracing::{info, Level};
use crate::app_state::AppState;
use slint::ComponentHandle;
use std::sync::Arc;

slint::include_modules!();

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP address of the phone (default is 127.0.0.1 for USB)
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,

    /// Port number (default 8125)
    #[arg(short, long, default_value_t = 8125)]
    port: u16,

    /// Transport method
    #[arg(short, long, value_enum, default_value_t = Transport::Usb)]
    transport: Transport,

    /// Audio codec
    #[arg(short, long, value_enum, default_value_t = Codec::Pcm)]
    codec: Codec,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Enable real-time scheduling (may require root or CAP_SYS_NICE)
    #[arg(long)]
    rt: bool,

    /// Start in CLI mode (no UI)
    #[arg(long)]
    cli: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Transport {
    Usb,
    Wifi,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Codec {
    Pcm,
    Opus,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let log_level = if args.debug {
        Level::DEBUG
    } else {
        Level::DEBUG // Force debug for now to diagnose the issue
    };

    let log_file = std::fs::File::create("/tmp/womic.log").expect("failed to create log file");

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(log_file)
        .with_ansi(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("Starting WOMIC-Linux client");

    if args.cli {
        return Err(anyhow::anyhow!("CLI mode not fully implemented in this refactor. Use UI."));
    }

    let ui = MainWindow::new()?;
    let ui_weak = ui.as_weak();
    
    let app_state = Arc::new(AppState::new(ui_weak.clone()));
    let app_state_clone = app_state.clone();

    ui.on_connect_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let ip = ui.get_ip_address().to_string();
            let port_str = ui.get_port().to_string();
            let is_usb = ui.get_is_usb();
            let codec_str = ui.get_selected_codec().to_string();

            let port = port_str.parse::<u16>().unwrap_or(8125);
            let codec = if codec_str == "Opus" { Codec::Opus } else { Codec::Pcm };

            app_state_clone.connect(ip, port, is_usb, codec);
        }
    });

    let app_state_disc = app_state.clone();
    ui.on_disconnect_clicked(move || {
        app_state_disc.disconnect();
    });

    // Handle Graceful Shutdown (REL-04)
    let app_state_shutdown = app_state.clone();
    tokio::spawn(async move {
        if let Ok(_) = tokio::signal::ctrl_c().await {
            info!("Received Ctrl+C, shutting down...");
            app_state_shutdown.disconnect();
            let _ = slint::invoke_from_event_loop(|| {
                slint::quit_event_loop().unwrap();
            });
        }
    });

    ui.run()?;
    
    // Final cleanup after UI exits
    app_state.disconnect();
    info!("WOMIC-Linux exited");

    Ok(())
}
