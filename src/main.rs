mod app;
mod database;
mod models;
mod ui;

use app::MemoApp;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let app = MemoApp::new().expect("Failed to initialize app");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native("Memo Stack", options, Box::new(|_cc| Ok(Box::new(app))))
}
