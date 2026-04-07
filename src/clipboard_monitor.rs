use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::db::{ClipKind, Db};

#[derive(Debug, Clone)]
pub enum ClipEvent {
    NewText(String),
    NewImage, // base64 already saved to DB
}

/// Spawns a blocking thread that polls the clipboard every `interval_ms` ms.
/// Sends events on `tx` whenever new content is detected.
pub fn start(db: Arc<std::sync::Mutex<Db>>, tx: mpsc::UnboundedSender<ClipEvent>) {
    let interval = Duration::from_millis(500);

    std::thread::spawn(move || {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to open clipboard: {e}");
                return;
            }
        };

        let mut last_text: Option<String> = None;
        let mut last_image_hash: Option<u64> = None;

        loop {
            std::thread::sleep(interval);

            // --- Text ---
            if let Ok(text) = clipboard.get_text() {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() && Some(&trimmed) != last_text.as_ref() {
                    last_text = Some(trimmed.clone());

                    if let Ok(db) = db.lock() {
                        match db.is_duplicate(&trimmed) {
                            Ok(true) => {
                                debug!("Duplicate text, skipping");
                            }
                            _ => {
                                let preview = make_text_preview(&trimmed);
                                if let Err(e) = db.insert(ClipKind::Text, &trimmed, &preview) {
                                    warn!("DB insert error: {e}");
                                } else {
                                    let _ = tx.send(ClipEvent::NewText(trimmed));
                                }
                            }
                        }
                    }
                }
            }

            // --- Image ---
            if let Ok(img) = clipboard.get_image() {
                let hash = image_hash(&img);
                if Some(hash) != last_image_hash {
                    last_image_hash = Some(hash);
                    if let Some(b64) = encode_image_to_png_b64(&img) {
                        if let Ok(db) = db.lock() {
                            let preview = format!("[Image {}x{}]", img.width, img.height);
                            if let Err(e) = db.insert(ClipKind::Image, &b64, &preview) {
                                warn!("DB insert image error: {e}");
                            } else {
                                let _ = tx.send(ClipEvent::NewImage);
                            }
                        }
                    }
                }
            }
        }
    });
}

fn make_text_preview(s: &str) -> String {
    let single: String = s.lines().collect::<Vec<_>>().join(" ");
    if single.len() > 80 {
        format!("{}…", &single[..79])
    } else {
        single
    }
}

fn image_hash(img: &ImageData) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    // Hash first 256 bytes of pixel data for speed
    let slice = &img.bytes[..img.bytes.len().min(256)];
    slice.hash(&mut h);
    img.width.hash(&mut h);
    img.height.hash(&mut h);
    h.finish()
}

fn encode_image_to_png_b64(img: &ImageData) -> Option<String> {
    use image::{DynamicImage, ImageBuffer, RgbaImage};
    let buf: RgbaImage = ImageBuffer::from_raw(
        img.width as u32,
        img.height as u32,
        img.bytes.to_vec(),
    )?;
    let dynamic = DynamicImage::ImageRgba8(buf);
    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    dynamic.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
    Some(B64.encode(&png_bytes))
}
