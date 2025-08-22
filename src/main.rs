mod app;
mod database;
mod icons;
mod models;
mod ui;

use app::MemoApp;
use eframe::egui;

// Include the custom font at compile time
static ATKINSON_FONT: &[u8] = include_bytes!(
    "../fonts/atkinson_hyperlegibile_mono/AtkinsonHyperlegibleMono-VariableFont_wght.ttf"
);
static PHOSPHOR_ICONS: &[u8] = include_bytes!("../fonts/phosphor_icons/regular/Phosphor.ttf");

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Memo Stack",
        options,
        Box::new(|cc| {
            // Load custom font
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "atkinson_mono".to_owned(),
                egui::FontData::from_static(ATKINSON_FONT).into(),
            );
            fonts.font_data.insert(
                "phosphor_icons".to_owned(),
                egui::FontData::from_static(PHOSPHOR_ICONS).into(),
            );

            // Set as default for all font families
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "atkinson_mono".to_owned());

            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "atkinson_mono".to_owned());

            // Create custom font family for icons
            fonts.families.insert(
                egui::FontFamily::Name("phosphor_icons".into()),
                vec!["phosphor_icons".to_owned()],
            );

            cc.egui_ctx.set_fonts(fonts);

            // Set custom theme colors
            let mut visuals = egui::Visuals::dark(); // Start with dark theme

            // Customize colors
            visuals.window_fill = egui::Color32::from_rgb(30, 30, 35); // Dark background
            visuals.panel_fill = egui::Color32::from_rgb(25, 25, 30); // Slightly darker panels
            visuals.faint_bg_color = egui::Color32::from_rgb(40, 40, 45); // Subtle backgrounds

            // Button colors
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(60, 60, 65);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(80, 80, 85);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(100, 100, 105);

            // Text colors
            visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(200, 200, 200);
            visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;

            // Accent color (for selections, highlights, etc.)
            visuals.selection.bg_fill = egui::Color32::from_rgb(70, 130, 180); // Steel blue
            visuals.selection.stroke.color = egui::Color32::from_rgb(100, 160, 210);

            // Border colors
            visuals.widgets.inactive.bg_stroke.color = egui::Color32::from_rgb(80, 80, 85);
            visuals.widgets.hovered.bg_stroke.color = egui::Color32::from_rgb(120, 120, 125);

            cc.egui_ctx.set_visuals(visuals);

            let app = MemoApp::new().expect("Failed to initialize app");
            Ok(Box::new(app))
        }),
    )
}
