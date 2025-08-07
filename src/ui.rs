use crate::app::MemoApp;
use crate::models::{ActiveTab, MemoData, MemoStatus};
use eframe::egui;

impl MemoApp {
    pub fn render_ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            //ui.heading("Memo Stack");

            // Tab buttons
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ActiveTab::Hot, "🔥 Hot");
                ui.selectable_value(&mut self.active_tab, ActiveTab::Cold, "❄ Cold");
                ui.selectable_value(&mut self.active_tab, ActiveTab::Done, "☑ Done");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .checkbox(&mut self.always_on_top, "📌")
                        .on_hover_text("Always on top")
                        .changed()
                    {
                        ctx.send_viewport_cmd(egui::viewport::ViewportCommand::WindowLevel(
                            if self.always_on_top {
                                egui::viewport::WindowLevel::AlwaysOnTop
                            } else {
                                egui::viewport::WindowLevel::Normal
                            },
                        ));
                    }
                });
            });

            ui.separator();

            match self.active_tab {
                ActiveTab::Hot => self.render_hot_tab(ui),
                ActiveTab::Cold => self.render_cold_tab(ui),
                ActiveTab::Done => self.render_done_tab(ui),
            }
        });
    }

    fn render_hot_tab(&mut self, ui: &mut egui::Ui) {
        // Update cold spotlight
        self.update_cold_spotlight();

        // Check if we have a spotlight to display
        let has_spotlight = self.config.cold_spotlight_interval_seconds > 0
            && self.current_spotlight_memo.is_some()
            && self
                .memos
                .get(&self.current_spotlight_memo.unwrap())
                .map(|memo| memo.status == MemoStatus::Cold)
                .unwrap_or(false);

        // Use bottom_up layout to make spotlight sticky at bottom
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            // Section 1: Memo Input (fixed max height, expandable with scrolling)
            ui.push_id("memo_input", |ui| {
                ui.group(|ui| {
                    ui.label("New memo (first line = title):");

                    let input_max_height = 120.0;
                    // Calculate current text height to determine if we need scrolling
                    let line_count = self.new_memo_text.lines().count().max(1);
                    let text_height =
                        (line_count as f32 * 14.0 + 20.0).min(input_max_height - 30.0);

                    egui::ScrollArea::vertical()
                        .max_height(input_max_height - 30.0)
                        .show(ui, |ui| {
                            ui.add_sized(
                                [ui.available_width(), text_height],
                                egui::TextEdit::multiline(&mut self.new_memo_text).hint_text(
                                    "Type your memo here...\nFirst line becomes the title",
                                ),
                            );
                        });

                    // Check for shift+enter to submit
                    let shift_enter_pressed =
                        ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.shift);

                    if ui.button("Add Memo").clicked() || shift_enter_pressed {
                        if !self.new_memo_text.trim().is_empty() {
                            let lines: Vec<&str> = self.new_memo_text.lines().collect();
                            let title = lines.first().unwrap_or(&"").to_string();
                            let body = if lines.len() > 1 {
                                lines[1..].join("\n")
                            } else {
                                String::new()
                            };

                            if let Err(e) = self.add_memo(title, body) {
                                eprintln!("Error adding memo: {}", e);
                            }
                            self.new_memo_text.clear();
                        }
                    }
                });
            });

            ui.add_space(10.0);

            // Section 2: Hot Memos (fills remaining space)
            ui.push_id("hot_memos", |ui| {
                ui.group(|ui| {
                    ui.label(format!(
                        "Hot memos: {}/{}",
                        self.hot_stack.len(),
                        self.config.max_hot_count
                    ));

                    // Calculate remaining height for hot memos
                    let remaining_height = if has_spotlight {
                        ui.available_height() - 120.0 // Reserve space for spotlight
                    } else {
                        ui.available_height() - 20.0 // Just some padding
                    };

                    egui::ScrollArea::vertical()
                        .max_height(remaining_height.max(100.0))
                        .show(ui, |ui| {
                            let memo_data: Vec<MemoData> = self
                                .hot_stack
                                .iter()
                                .filter_map(|id| self.memos.get(id).cloned())
                                .collect();

                            for memo in memo_data {
                                self.render_memo_item(ui, &memo, true);
                            }
                        });
                });
            });

            // Section 3: Cold Spotlight (sticky at bottom)
            if has_spotlight {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.push_id("cold_spotlight", |ui| {
                        ui.group(|ui| {
                            if let Some(spotlight_memo) = self
                                .current_spotlight_memo
                                .and_then(|id| self.memos.get(&id).cloned())
                            {
                                self.render_memo_item(ui, &spotlight_memo, false);
                                ui.separator();
                                ui.label(format!(
                                    "💡 Cold Spotlight (refreshes every {} seconds):",
                                    self.config.cold_spotlight_interval_seconds
                                ));
                            }
                        });
                    });
                });
            }
        });
    }

    fn render_cold_tab(&mut self, ui: &mut egui::Ui) {
        // Search bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.add_sized(
                [ui.available_width() - 60.0, 20.0],
                egui::TextEdit::singleline(&mut self.cold_search).hint_text("Search cold memos..."),
            );
        });

        ui.separator();

        let cold_memos = self.get_filtered_memos(MemoStatus::Cold, &self.cold_search);
        ui.label(format!("Cold memos: {}", cold_memos.len()));

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (_, memo) in cold_memos {
                self.render_memo_item(ui, &memo, false);
            }
        });
    }

    fn render_done_tab(&mut self, ui: &mut egui::Ui) {
        // Search bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.add_sized(
                [ui.available_width() - 60.0, 20.0],
                egui::TextEdit::singleline(&mut self.done_search).hint_text("Search done memos..."),
            );
        });

        ui.separator();

        let done_memos = self.get_filtered_memos(MemoStatus::Done, &self.done_search);
        ui.label(format!("Done memos: {}", done_memos.len()));

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (_, memo) in done_memos {
                self.render_memo_item(ui, &memo, false);
            }
        });
    }

    fn render_memo_item(&mut self, ui: &mut egui::Ui, memo: &MemoData, is_hot: bool) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // Hot tab specific controls
                if is_hot {
                    // Shift up button (not for top item)
                    if let Some(pos) = self.hot_stack.iter().position(|&x| x == memo.id) {
                        if pos > 0 {
                            let shift_pressed = ui.input(|i| i.modifiers.shift);
                            let button_text = if shift_pressed { "⇑" } else { "⌃" };
                            let hover_text = if shift_pressed {
                                "Move to Top"
                            } else {
                                "Shift Up"
                            };
                            if ui.button(button_text).on_hover_text(hover_text).clicked() {
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
                    if ui.button("❄").on_hover_text("Move to Cold").clicked() {
                        if let Err(e) = self.move_to_cold(memo.id) {
                            eprintln!("Error moving to cold: {}", e);
                        }
                    }
                } else {
                    // Cold/Done tab - move to hot button
                    if memo.status != MemoStatus::Done {
                        if ui.button("🔥").on_hover_text("Move to Hot").clicked() {
                            if let Err(e) = self.move_to_hot(memo.id) {
                                eprintln!("Error moving to hot: {}", e);
                            }
                        }
                    }
                }

                // Expand button (only if has body)
                if !memo.body.is_empty() {
                    let expand_text = if memo.expanded { "−" } else { "+" };
                    if ui.button(expand_text).clicked() {
                        if let Some(memo_mut) = self.memos.get_mut(&memo.id) {
                            memo_mut.expanded = !memo_mut.expanded;
                        }
                    }
                }

                // Replace button (only for hot memos)
                if is_hot {
                    if ui.button("✎").on_hover_text("Edit / Replace").clicked() {
                        if let Err(e) = self.replace_memo(memo.id) {
                            eprintln!("Error replacing memo: {}", e);
                        }
                    }
                }

                // Status action button
                match memo.status {
                    MemoStatus::Hot | MemoStatus::Cold => {
                        if ui.button("✓").clicked() {
                            if let Err(e) = self.move_to_done(memo.id) {
                                eprintln!("Error moving to done: {}", e);
                            }
                        }
                    }
                    MemoStatus::Done => {
                        if ui.button("☑").on_hover_text("Move to Done").clicked() {
                            if let Err(e) = self.delete_memo(memo.id) {
                                eprintln!("Error deleting memo: {}", e);
                            }
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

    pub fn get_filtered_memos(&self, status: MemoStatus, search: &str) -> Vec<(i32, MemoData)> {
        let mut memos: Vec<(i32, MemoData)> = self
            .memos
            .iter()
            .filter(|(_, memo)| memo.status == status)
            .map(|(&id, memo)| (id, memo.clone()))
            .collect();

        if !search.trim().is_empty() {
            let search_lower = search.to_lowercase();
            memos.retain(|(_, memo)| {
                memo.title.to_lowercase().contains(&search_lower)
                    || memo.body.to_lowercase().contains(&search_lower)
            });
        }

        // Sort by creation date (newest first) for cold, by moved_to_done_date for done
        match status {
            MemoStatus::Cold => {
                memos.sort_by(|a, b| b.1.creation_date.cmp(&a.1.creation_date));
            }
            MemoStatus::Done => {
                memos.sort_by(
                    |a, b| match (a.1.moved_to_done_date, b.1.moved_to_done_date) {
                        (Some(a_date), Some(b_date)) => b_date.cmp(&a_date),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => b.1.creation_date.cmp(&a.1.creation_date),
                    },
                );
            }
            _ => {}
        }

        memos
    }
}
