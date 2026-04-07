use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct HotkeyManager {
    _manager: GlobalHotKeyManager,
}

/// Registers Super+V as a global hotkey. On Wayland this requires either
/// XWayland or compositor-level support. If registration fails, a warning
/// is logged and the hotkey is silently disabled (tray click still works).
///
/// Returns the manager (must stay alive) and starts a listener thread that
/// sends `()` on `tx` each time the hotkey fires.
pub fn register(tx: mpsc::UnboundedSender<()>) -> Option<HotkeyManager> {
    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            warn!("Could not create hotkey manager (Wayland may not support it): {e}");
            warn!("Use tray icon to open clipboard instead.");
            return None;
        }
    };

    // Super + V
    let hotkey = HotKey::new(Some(Modifiers::SUPER), Code::KeyV);
    let hotkey_id = hotkey.id();
    match manager.register(hotkey) {
        Ok(_) => {
            info!("Global hotkey Super+V registered");
        }
        Err(e) => {
            warn!("Failed to register Super+V hotkey: {e}");
            warn!("Tip: On COSMIC Wayland, add a custom keybinding in Settings → Keyboard → \
                   Custom Shortcuts that runs: clipmgr --show");
            return None;
        }
    }

    // Listener thread
    std::thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();
        loop {
            if let Ok(event) = receiver.recv() {
                if event.id == hotkey_id {
                    let _ = tx.send(());
                }
            }
        }
    });

    Some(HotkeyManager { _manager: manager })
}
