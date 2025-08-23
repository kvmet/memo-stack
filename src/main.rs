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
            // Configure fonts
            let mut fonts = egui::FontDefinitions::default();
            ui::theme::configure_fonts(&mut fonts, ATKINSON_FONT, PHOSPHOR_ICONS);
            cc.egui_ctx.set_fonts(fonts);

            // Configure theme
            let visuals = ui::theme::configure_visuals();
            cc.egui_ctx.set_visuals(visuals);

            let app = MemoApp::new().expect("Failed to initialize app");
            Ok(Box::new(app))
        }),
    )
}
