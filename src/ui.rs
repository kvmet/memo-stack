use crate::app::MemoApp;
use crate::models::{ActiveTab, MemoData, MemoStatus};
use eframe::egui;

impl MemoApp {
    pub fn render_ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Memo Stack");

            // Tab buttons
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ActiveTab::Hot, "🔥 Hot");
                ui.selectable_value(&mut self.active_tab, ActiveTab::Cold, "❄ Cold");
                ui.selectable_value(&mut self.active_tab, ActiveTab::Done, "✓ Done");
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
        // Input section (only in Hot tab)
        ui.label("New memo (first line = title):");
        let font_id = egui::FontId::new(14.0, self.get_font_family());
        ui.add(egui::TextEdit::multiline(&mut self.new_memo_text).font(font_id.clone()));

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

        ui.separator();

        // Display hot stack info
        ui.label(format!(
            "Hot memos: {}/{}",
            self.hot_stack.len(),
            self.config.max_hot_count
        ));

        // Display hot memos in stack order
        egui::ScrollArea::vertical().show(ui, |ui| {
            let memo_data: Vec<MemoData> = self
                .hot_stack
                .iter()
                .filter_map(|id| self.memos.get(id).cloned())
                .collect();

            for memo in memo_data {
                self.render_memo_item(ui, &memo, true);
            }
        });
    }

    fn render_cold_tab(&mut self, ui: &mut egui::Ui) {
        // Search bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.add(
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
            ui.add(
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
            ui.horizontal(|ui| {
                // Hot tab specific controls
                if is_hot {
                    // Shift up button (not for top item)
                    if let Some(pos) = self.hot_stack.iter().position(|&x| x == memo.id) {
                        if pos > 0 {
                            let shift_pressed = ui.input(|i| i.modifiers.shift);
                            let button_text = if shift_pressed { "^^" } else { "^" };
                            if ui.button(button_text).clicked() {
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
                    if ui.button("❄").clicked() {
                        if let Err(e) = self.move_to_cold(memo.id) {
                            eprintln!("Error moving to cold: {}", e);
                        }
                    }
                } else {
                    // Cold/Done tab - move to hot button
                    if memo.status != MemoStatus::Done {
                        if ui.button("🔥").clicked() {
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
                    if ui.button("⟲").clicked() {
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
                        if ui.button("🗑").clicked() {
                            if let Err(e) = self.delete_memo(memo.id) {
                                eprintln!("Error deleting memo: {}", e);
                            }
                        }
                    }
                }

                // Title
                let font_id = egui::FontId::new(14.0, self.get_font_family());
                ui.add(egui::Label::new(
                    egui::RichText::new(&memo.title).font(font_id),
                ));
            });

            // Show body if expanded
            if memo.expanded && !memo.body.is_empty() {
                ui.separator();
                let font_id = egui::FontId::new(14.0, self.get_font_family());
                ui.add(egui::Label::new(
                    egui::RichText::new(&memo.body).font(font_id),
                ));
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

    pub fn get_font_family(&self) -> egui::FontFamily {
        match self.config.font_family.as_str() {
            "monospace" => egui::FontFamily::Monospace,
            _ => egui::FontFamily::Proportional,
        }
    }
}
