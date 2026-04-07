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
        // Uses a standard freedesktop icon; falls back gracefully
        "edit-paste".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: "edit-paste".into(),
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
        let _ = self.tx.send(TrayAction::ShowWindow);
    }
}

/// Starts the system tray in its own thread. Returns a handle that can be used
/// to update the tray (e.g., refresh the item count).
pub fn start(
    db: Arc<Mutex<Db>>,
    tx: mpsc::UnboundedSender<TrayAction>,
) -> TrayService<ClipTray> {
    let tray = ClipTray { tx, db };
    TrayService::new(tray)
}
