mod app;
mod editor;
mod file_browser;
mod sidebar;
mod toolbar;

pub use app::ExploreApp;

use anyhow::Result;
use std::path::PathBuf;

pub fn run(start_path: PathBuf) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ExploreRust")
            .with_inner_size([1200.0, 750.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(eframe::icon_data::from_png_bytes(ICON).unwrap_or_default()),
        ..Default::default()
    };

    eframe::run_native(
        "ExploreRust",
        options,
        Box::new(|cc| Ok(Box::new(ExploreApp::new(cc, start_path)))),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {}", e))
}

// Embed a simple icon (16×16 blue folder icon as PNG bytes)
const ICON: &[u8] = include_bytes!("../../assets/icon.png");
