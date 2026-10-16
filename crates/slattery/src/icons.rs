//! Window icon helpers.
//!
//! The SlateScript logo is embedded in the binary and used as the
//! default window icon. Apps loaded via `slattery run` can override
//! it with their own `assets/logo.png`.

use std::path::Path;

/// The embedded SlateScript logo bytes. Same file `install.rs` uses
/// when scaffolding new apps.
pub const LOGO_PNG: &[u8] = include_bytes!("logo.png");

/// Decode a PNG file into an egui icon, resized to 64×64.
pub fn load_from_png(path: &Path) -> Result<egui::IconData, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("cannot read '{}': {}", path.display(), e))?;
    decode_png(&bytes)
}

/// The default SlateScript icon. Decoded once per call — cheap enough
/// that we don't bother caching.
pub fn default_icon() -> Option<egui::IconData> {
    decode_png(LOGO_PNG).ok()
}

fn decode_png(bytes: &[u8]) -> Result<egui::IconData, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("image decode: {}", e))?;
    let img = img.resize_exact(64, 64, image::imageops::FilterType::Lanczos3);
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}