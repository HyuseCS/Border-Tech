mod audio;
mod protocol;
mod app_state;

use clap::Parser;
use tracing::{info, Level};
use crate::app_state::AppState;
use slint::ComponentHandle;
use std::sync::Arc;

slint::include_modules!();

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port number to listen on (default 47999)
    #[arg(short, long, default_value_t = 47999)]
    port: u16,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

pub fn get_local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    // Connect to a public IP to determine which local network interface is routeable.
    // This doesn't actually send any packets over the internet.
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let log_level = if args.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let log_file = std::fs::File::create("/tmp/projectm.log").expect("failed to create log file");

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(log_file)
        .with_ansi(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("Starting Project-M Client");

    let ui = MainWindow::new()?;
    let ui_weak = ui.as_weak();
    
    // Set local IP and port in the UI
    let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1 (Offline)".to_string());
    ui.set_local_ip(local_ip.into());
    ui.set_port(args.port.to_string().into());

    let app_state = Arc::new(AppState::new(ui_weak.clone()));
    let app_state_clone = app_state.clone();

    ui.on_connect_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let port_str = ui.get_port().to_string();
            let is_usb = ui.get_is_usb();
            let port = port_str.parse::<u16>().unwrap_or(47999);

            app_state_clone.connect(port, is_usb);
        }
    });

    let app_state_disc = app_state.clone();
    ui.on_disconnect_clicked(move || {
        app_state_disc.disconnect();
    });

    // Handle Graceful Shutdown
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
    info!("Project-M Client exited");

    Ok(())
}
