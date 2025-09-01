use crate::app::MemoApp;
use crate::icons;
use crate::models::ActiveTab;

use eframe::egui;

impl MemoApp {
    pub fn render_ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Tab buttons
            ui.horizontal(|ui| {
                self.render_tab_button(ui, ActiveTab::Hot, icons::HOT, "Hot");
                self.render_tab_button(ui, ActiveTab::Cold, icons::COLD, "Cold");
                self.render_tab_button(ui, ActiveTab::Done, icons::DONE, "Done");
                self.render_tab_button(ui, ActiveTab::Delayed, icons::DELAY, "Delayed");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    use std::sync::Once;
                    static INIT: Once = Once::new();

                    let checkbox_response = ui.checkbox(&mut self.always_on_top, "   ");

                    // Draw the settings icon on top of the checkbox
                    let icon_pos = checkbox_response.rect.left_center() + egui::vec2(20.0, 0.0);
                    ui.painter().text(
                        icon_pos,
                        egui::Align2::LEFT_CENTER,
                        icons::ALWAYS_ON_TOP,
                        egui::FontId::new(16.0, egui::FontFamily::Name("phosphor_icons".into())),
                        ui.visuals().text_color(),
                    );

                    let checkbox_changed =
                        checkbox_response.on_hover_text("Always on top").changed();

                    if checkbox_changed {
                        ctx.send_viewport_cmd(egui::viewport::ViewportCommand::WindowLevel(
                            if self.always_on_top {
                                egui::viewport::WindowLevel::AlwaysOnTop
                            } else {
                                egui::viewport::WindowLevel::Normal
                            },
                        ));

                        // Save app state to database
                        let _ = self.save_app_state();
                    }

                    // Apply initial state on first render
                    INIT.call_once(|| {
                        if self.always_on_top {
                            ctx.send_viewport_cmd(egui::viewport::ViewportCommand::WindowLevel(
                                egui::viewport::WindowLevel::AlwaysOnTop,
                            ));
                        }
                    });
                });
            });

            ui.separator();

            match self.active_tab {
                ActiveTab::Hot => self.render_hot_tab(ui),
                ActiveTab::Cold => self.render_cold_tab(ui),
                ActiveTab::Done => self.render_done_tab(ui),
                ActiveTab::Delayed => self.render_delayed_tab(ui),
            }
        });
    }

    fn render_tab_button(&mut self, ui: &mut egui::Ui, tab: ActiveTab, icon: &str, text: &str) {
        let is_selected = self.active_tab == tab;

        let response = icons::tab_button_with_icon(ui, icon, text, is_selected);

        if response.clicked() {
            self.active_tab = tab;
        }
    }
}
