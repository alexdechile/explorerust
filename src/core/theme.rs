/// Detects if the system is in dark or light mode.
pub fn is_dark_mode() -> bool {
    match dark_light::detect() {
        dark_light::Mode::Dark => true,
        dark_light::Mode::Light => false,
        dark_light::Mode::Default => true, // default to dark
    }
}

/// Try to read the system accent color (GNOME/KDE).
/// Returns None if not detectable.
pub fn accent_color() -> Option<egui::Color32> {
    // Try GNOME accent via gsettings
    if let Ok(out) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "accent-color"])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return match s.trim_matches('\'') {
            "blue" => Some(egui::Color32::from_rgb(53, 132, 228)),
            "teal" => Some(egui::Color32::from_rgb(42, 161, 152)),
            "green" => Some(egui::Color32::from_rgb(46, 194, 126)),
            "yellow" => Some(egui::Color32::from_rgb(249, 240, 107)),
            "orange" => Some(egui::Color32::from_rgb(255, 120, 0)),
            "red" => Some(egui::Color32::from_rgb(224, 27, 36)),
            "pink" => Some(egui::Color32::from_rgb(220, 140, 200)),
            "purple" => Some(egui::Color32::from_rgb(145, 65, 172)),
            "slate" => Some(egui::Color32::from_rgb(111, 131, 150)),
            _ => None,
        };
    }
    None
}

pub fn apply_theme(ctx: &egui::Context, dark: bool, accent: Option<egui::Color32>) {
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    if let Some(color) = accent {
        visuals.selection.bg_fill = color.linear_multiply(0.5);
        visuals.hyperlink_color = color;
        visuals.widgets.active.bg_fill = color;
        visuals.widgets.hovered.bg_fill =
            egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 180);
    }

    ctx.set_visuals(visuals);
}
