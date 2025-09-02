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
    // Load saved window state from database
    let (window_width, window_height, window_x, window_y) =
        database::load_window_state().unwrap_or((800.0, 600.0, None, None));

    // Build viewport with saved size and position
    let mut viewport_builder =
        egui::ViewportBuilder::default().with_inner_size([window_width, window_height]);

    if let (Some(x), Some(y)) = (window_x, window_y) {
        viewport_builder = viewport_builder.with_position([x, y]);
    }

    let options = eframe::NativeOptions {
        viewport: viewport_builder,
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
