mod clipboard_monitor;
mod db;
mod hotkey;
mod tray;
mod ui;

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::info;

use db::Db;
use tray::TrayAction;

fn main() {
    // Logging
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("clipmgr=info".parse().unwrap()),
        )
        .init();

    // Handle --show flag: open window against the shared DB.
    // Useful for COSMIC custom keybinding: set command to `clipmgr --show`
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("--show") {
        let db = Arc::new(Mutex::new(Db::open().expect("DB open failed")));
        ui::run(db).expect("UI failed");
        return;
    }

    // Multi-threaded Tokio runtime for the background daemon
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Tokio runtime failed");

    rt.block_on(async_main());
}

async fn async_main() {
    info!("clipmgr starting…");

    let db = Arc::new(Mutex::new(Db::open().expect("Failed to open database")));

    // clipboard monitor → main
    let (clip_tx, mut clip_rx) = mpsc::unbounded_channel::<clipboard_monitor::ClipEvent>();
    // tray → main
    let (tray_tx, mut tray_rx) = mpsc::unbounded_channel::<TrayAction>();
    // hotkey → main
    let (hotkey_tx, mut hotkey_rx) = mpsc::unbounded_channel::<()>();

    clipboard_monitor::start(db.clone(), clip_tx);
    info!("Clipboard monitor started");

    tray::start(db.clone(), tray_tx);
    info!("System tray started");

    let _hotkey_mgr = hotkey::register(hotkey_tx);

    info!("Running. Press Super+V or click the tray icon to open.");

    loop {
        tokio::select! {
            Some(event) = clip_rx.recv() => {
                match event {
                    clipboard_monitor::ClipEvent::NewText(t) => {
                        info!("Saved text: {}…", &t[..t.len().min(40)]);
                    }
                    clipboard_monitor::ClipEvent::NewImage => {
                        info!("Saved image");
                    }
                }
            }
            Some(action) = tray_rx.recv() => {
                match action {
                    TrayAction::ShowWindow => {
                        let _ = std::process::Command::new(
                            std::env::current_exe().unwrap_or_else(|_| "clipmgr".into()),
                        )
                        .arg("--show")
                        .spawn();
                    }
                    TrayAction::ClearHistory => {
                        if let Ok(db) = db.lock() {
                            let _ = db.clear_unpinned();
                        }
                        info!("History cleared");
                    }
                    TrayAction::Quit => {
                        info!("Quitting.");
                        std::process::exit(0);
                    }
                }
            }
            Some(()) = hotkey_rx.recv() => {
                let _ = std::process::Command::new(
                    std::env::current_exe().unwrap_or_else(|_| "clipmgr".into()),
                )
                .arg("--show")
                .spawn();
            }
        }
    }
}
