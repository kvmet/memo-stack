use crate::app::MemoApp;
use crate::icons;
use crate::models::ActiveTab;

use eframe::egui;

impl MemoApp {
    pub fn render_ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Tab buttons
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::Hot,
                    icons::icon_with_text(icons::HOT, "Hot"),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::Cold,
                    icons::icon_with_text(icons::COLD, "Cold"),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::Done,
                    icons::icon_with_text(icons::DONE, "Done"),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::Delayed,
                    icons::icon_with_text(icons::DELAY, "Delayed"),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    use std::sync::Once;
                    static INIT: Once = Once::new();

                    let checkbox_changed = ui
                        .checkbox(
                            &mut self.always_on_top,
                            icons::icon_with_text(icons::SETTINGS, ""),
                        )
                        .on_hover_text("Always on top")
                        .changed();

                    if checkbox_changed {
                        ctx.send_viewport_cmd(egui::viewport::ViewportCommand::WindowLevel(
                            if self.always_on_top {
                                egui::viewport::WindowLevel::AlwaysOnTop
                            } else {
                                egui::viewport::WindowLevel::Normal
                            },
                        ));
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
}
