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

            let app = MemoApp::new().expect("Failed to initialize app");
            Ok(Box::new(app))
        }),
    )
}
