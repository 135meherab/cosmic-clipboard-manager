use ksni::{menu::*, Tray, TrayService};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::db::Db;

pub enum TrayAction {
    ShowWindow,
    ClearHistory,
    Quit,
}

struct ClipTray {
    tx: mpsc::UnboundedSender<TrayAction>,
    db: Arc<Mutex<Db>>,
}

impl Tray for ClipTray {
    fn id(&self) -> String {
        "clipmgr".into()
    }

    fn title(&self) -> String {
        "Clipboard Manager".into()
    }

    fn icon_name(&self) -> String {
        "edit-paste-symbolic".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: "edit-paste-symbolic".into(),
            icon_pixmap: vec![],
            title: "Clipboard Manager".into(),
            description: "Click to open clipboard history".into(),
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let count = self
            .db
            .lock()
            .map(|db| db.all().map(|v| v.len()).unwrap_or(0))
            .unwrap_or(0);

        vec![
            MenuItem::Standard(StandardItem {
                label: format!("Open Clipboard ({count} items)"),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayAction::ShowWindow);
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Clear History".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayAction::ClearHistory);
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Quit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayAction::Quit);
                }),
                ..Default::default()
            }),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        // Spawn clipmgr --show as a child process so iced runs on its own main thread
        let _ = std::process::Command::new(
            std::env::current_exe().unwrap_or_else(|_| "clipmgr".into()),
        )
        .arg("--show")
        .spawn();
    }
}

/// Starts the system tray in its own background thread.
pub fn start(db: Arc<Mutex<Db>>, tx: mpsc::UnboundedSender<TrayAction>) {
    let tray = ClipTray { tx, db };
    TrayService::new(tray).spawn();
}
