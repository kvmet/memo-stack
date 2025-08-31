use crate::app::MemoApp;
use crate::icons;
use crate::models::{MemoData, MemoStatus};
use eframe::egui;

impl MemoApp {
    pub fn render_memo_item(&mut self, ui: &mut egui::Ui, memo: &MemoData, is_hot: bool) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // Hot tab specific controls
                if is_hot {
                    // Shift up button (not for top item)
                    if let Some(pos) = self.hot_stack.iter().position(|&x| x == memo.id) {
                        if pos > 0 {
                            let shift_pressed = ui.input(|i| i.modifiers.shift);
                            let button_icon = if shift_pressed {
                                icons::MOVE_TO_TOP
                            } else {
                                icons::MOVE_UP
                            };
                            let hover_text = if shift_pressed {
                                "Move to Top"
                            } else {
                                "Shift Up"
                            };
                            if ui
                                .button(icons::icon_text(button_icon))
                                .on_hover_text(hover_text)
                                .clicked()
                            {
                                if shift_pressed {
                                    if let Err(e) = self.move_to_top_in_hot(memo.id) {
                                        eprintln!("Error moving to top: {}", e);
                                    }
                                } else {
                                    if let Err(e) = self.shift_up_in_hot(memo.id) {
                                        eprintln!("Error shifting memo: {}", e);
                                    }
                                }
                            }
                        }
                    }

                    // Move to cold button
                    if ui
                        .button(icons::icon_text(icons::COLD))
                        .on_hover_text("Move to Cold")
                        .clicked()
                    {
                        if let Err(e) = self.move_to_cold(memo.id) {
                            eprintln!("Error moving to cold: {}", e);
                        }
                    }
                } else {
                    // Cold/Done tab - move to hot button
                    if memo.status != MemoStatus::Done {
                        if ui
                            .button(icons::icon_text(icons::HOT))
                            .on_hover_text("Move to Hot")
                            .clicked()
                        {
                            if let Err(e) = self.move_to_hot(memo.id) {
                                eprintln!("Error moving to hot: {}", e);
                            }
                        }
                    }
                }

                // Replace button (only for hot memos)
                if is_hot {
                    if ui
                        .button(icons::icon_text(icons::EDIT))
                        .on_hover_text("Edit / Replace")
                        .clicked()
                    {
                        if let Err(e) = self.replace_memo(memo.id) {
                            eprintln!("Error replacing memo: {}", e);
                        }
                    }
                }

                // Status action button
                match memo.status {
                    MemoStatus::Hot | MemoStatus::Cold => {
                        if ui
                            .button(icons::icon_text(icons::DONE))
                            .on_hover_text("Move to Done")
                            .clicked()
                        {
                            if let Err(e) = self.move_to_done(memo.id) {
                                eprintln!("Error moving to done: {}", e);
                            }
                        }
                    }
                    MemoStatus::Done => {
                        if ui
                            .button(icons::icon_text(icons::DELETE))
                            .on_hover_text("Delete")
                            .clicked()
                        {
                            if let Err(e) = self.delete_memo(memo.id) {
                                eprintln!("Error deleting memo: {}", e);
                            }
                        }
                    }
                    MemoStatus::Delayed => {
                        if ui
                            .button(icons::icon_text(icons::HOT))
                            .on_hover_text("Move to Hot")
                            .clicked()
                        {
                            if let Err(e) = self.move_to_hot(memo.id) {
                                eprintln!("Error moving to hot: {}", e);
                            }
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                // Expand button (only if has body)
                if !memo.body.is_empty() {
                    let expand_icon = if memo.expanded {
                        icons::COLLAPSE
                    } else {
                        icons::EXPAND
                    };
                    if ui.button(icons::icon_text(expand_icon)).clicked() {
                        if let Some(memo_mut) = self.memos.get_mut(&memo.id) {
                            memo_mut.expanded = !memo_mut.expanded;
                        }
                    }
                }

                // Title
                ui.add(egui::Label::new(&memo.title).wrap());
            });

            // Show body if expanded
            if memo.expanded && !memo.body.is_empty() {
                ui.separator();
                ui.add(egui::Label::new(&memo.body).wrap());
            }

            // Show dates
            ui.horizontal(|ui| {
                ui.small(format!(
                    "Created: {}",
                    memo.creation_date.format("%Y-%m-%d %H:%M")
                ));
                if let Some(done_date) = memo.moved_to_done_date {
                    ui.small(format!("Done: {}", done_date.format("%Y-%m-%d %H:%M")));
                }
            });
        });
    }
}
