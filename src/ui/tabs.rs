use crate::app::MemoApp;
use crate::icons;
use crate::models::MemoStatus;

use chrono::Utc;
use eframe::egui;
use rusqlite::Result;

impl MemoApp {
    pub fn render_hot_tab(&mut self, ui: &mut egui::Ui) {
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
                let input_max_height = 180.0;
                // Calculate current text height to determine if we need scrolling
                let line_count = self.new_memo_text.lines().count().max(1);
                let text_height = (line_count as f32 * 14.0 + 20.0).min(input_max_height - 30.0);

                egui::ScrollArea::vertical()
                    .max_height(input_max_height - 30.0)
                    .show(ui, |ui| {
                        let output = ui.input_mut(|input| {
                            // Consume Tab keys before TextEdit gets them
                            let shift_tab =
                                input.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
                            let tab = input.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                            (shift_tab, tab)
                        });

                        let text_edit_id = ui.id().with("memo_text_edit");
                        let text_edit = egui::TextEdit::multiline(&mut self.new_memo_text)
                            .hint_text("Enter memo...")
                            .desired_rows((text_height / 14.0) as usize)
                            .desired_width(ui.available_width())
                            .lock_focus(true)
                            .id(text_edit_id);

                        let text_output = text_edit.show(ui);
                        let response = text_output.response;

                        // Handle tab key input for indentation when text area has focus
                        if response.has_focus() {
                            let (shift_tab_pressed, tab_pressed) = output;
                            if shift_tab_pressed {
                                // Shift+Tab: remove indentation
                                if let Some(cursor_range) = text_output.cursor_range {
                                    let char_range = cursor_range.as_sorted_char_range();
                                    let cursor_pos = cursor_range.primary.index;

                                    if char_range.is_empty() {
                                        // No selection - outdent current line
                                        self.handle_tab_indent(cursor_pos, false);
                                    } else {
                                        // Selection exists - outdent all selected lines
                                        self.handle_multiline_indent(
                                            char_range.start,
                                            char_range.end,
                                            false,
                                            ui,
                                            text_edit_id,
                                        );
                                    }
                                }
                            } else if tab_pressed {
                                // Tab: add indentation or insert spaces
                                if let Some(cursor_range) = text_output.cursor_range {
                                    let char_range = cursor_range.as_sorted_char_range();
                                    let cursor_pos = cursor_range.primary.index;

                                    if char_range.is_empty() {
                                        // No selection - insert spaces at cursor
                                        self.handle_tab_insert(cursor_pos, ui, text_edit_id);
                                    } else {
                                        // Selection exists - indent all selected lines
                                        self.handle_multiline_indent(
                                            char_range.start,
                                            char_range.end,
                                            true,
                                            ui,
                                            text_edit_id,
                                        );
                                    }
                                }
                            }
                        }

                        // Request immediate repaint if we handled any tab input
                        let (shift_tab_pressed, tab_pressed) = output;
                        if (shift_tab_pressed || tab_pressed) && response.has_focus() {
                            ui.ctx().request_repaint();
                        }

                        if response.changed() || response.has_focus() {
                            ui.ctx().request_repaint();
                        }
                    });

                // Buttons row
                ui.horizontal(|ui| {
                    // Add memo button
                    let add_enabled = !self.new_memo_text.trim().is_empty();
                    if (icons::button_with_icon(ui, icons::ADD, "Add to Hot", add_enabled)
                        .clicked()
                        || (ui.input(|i| {
                            i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl && add_enabled
                        })))
                        && add_enabled
                    {
                        if let Err(e) = self.add_parsed_memo(None) {
                            eprintln!("Error adding memo: {}", e);
                        }
                    }

                    // Delay controls
                    ui.separator();
                    ui.label("Delay:");

                    // Delay input
                    let _delay_response = ui.add_sized(
                        [50.0, 20.0],
                        egui::TextEdit::singleline(&mut self.delay_input).hint_text("HH:MM"),
                    );

                    // Quick delay buttons
                    if ui.small_button("15m").clicked() {
                        self.adjust_delay_input(15);
                    }
                    if ui.small_button("1h").clicked() {
                        self.adjust_delay_input(60);
                    }
                    if ui.small_button("4h").clicked() {
                        self.adjust_delay_input(240);
                    }

                    // Add delayed button
                    let delay_enabled =
                        !self.new_memo_text.trim().is_empty() && self.parse_delay_input().is_some();
                    if icons::button_with_icon(ui, icons::DELAY, "Add Delayed", delay_enabled)
                        .clicked()
                        && delay_enabled
                    {
                        if let Err(e) = self.add_parsed_memo(self.parse_delay_input()) {
                            eprintln!("Error adding delayed memo: {}", e);
                        }
                    }
                });
            });

            ui.separator();

            // Section 2: Hot Stack
            ui.push_id("hot_stack", |ui| {
                let available_height =
                    ui.available_height() - if has_spotlight { 150.0 } else { 0.0 } - 10.0;

                ui.label(format!("Hot stack: {}", self.hot_stack.len()));

                egui::ScrollArea::vertical()
                    .max_height(available_height)
                    .show(ui, |ui| {
                        for &memo_id in &self.hot_stack.clone() {
                            if let Some(memo) = self.memos.get(&memo_id) {
                                let memo_clone = memo.clone();
                                self.render_memo_item(ui, &memo_clone, true);
                            }
                        }
                    });
            });

            // Section 3: Cold Spotlight (if enabled and available)
            if has_spotlight {
                ui.separator();
                ui.push_id("cold_spotlight", |ui| {
                    if let Some(spotlight_id) = self.current_spotlight_memo {
                        if let Some(memo) = self.memos.get(&spotlight_id) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.add(egui::Label::new(icons::icon_text(icons::COLD)));
                                ui.label("Cold Spotlight:");
                            });
                            let memo_clone = memo.clone();
                            self.render_memo_item(ui, &memo_clone, false);
                        }
                    }
                });
            }
        });
    }

    pub fn render_cold_tab(&mut self, ui: &mut egui::Ui) {
        // Search bar
        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add(egui::Label::new(icons::icon_text(icons::SEARCH)));
                ui.label("Search:");
            });
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

    pub fn render_done_tab(&mut self, ui: &mut egui::Ui) {
        // Search bar
        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add(egui::Label::new(icons::icon_text(icons::SEARCH)));
                ui.label("Search:");
            });
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

    pub fn render_delayed_tab(&mut self, ui: &mut egui::Ui) {
        let delayed_ids: Vec<i32> = self
            .memos
            .iter()
            .filter(|(_, memo)| memo.status == MemoStatus::Delayed)
            .map(|(&id, _)| id)
            .collect();

        ui.label(format!("Delayed memos: {}", delayed_ids.len()));

        egui::ScrollArea::vertical().show(ui, |ui| {
            for id in delayed_ids {
                if let Some(memo) = self.memos.get(&id) {
                    let memo_clone = memo.clone();

                    // Show timing information
                    if let Some(delay_minutes) = memo.delay_minutes {
                        let now = Utc::now();
                        let promotion_time =
                            memo.creation_date + chrono::Duration::minutes(delay_minutes as i64);

                        if now >= promotion_time {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.add(egui::Label::new(icons::icon_text(icons::HOT)));
                                ui.label(&format!("Ready to promote: {}", memo.title));
                            });
                        } else {
                            let remaining = promotion_time - now;
                            let total_seconds = remaining.num_seconds();
                            let hours = total_seconds / 3600;
                            let minutes = (total_seconds % 3600) / 60;
                            let seconds = total_seconds % 60;

                            if hours > 0 {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui.add(egui::Label::new(icons::icon_text(icons::DELAY)));
                                    ui.label(&format!(
                                        "{} (ready in {}h {}m {}s)",
                                        memo.title, hours, minutes, seconds
                                    ));
                                });
                            } else if minutes > 0 {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui.add(egui::Label::new(icons::icon_text(icons::DELAY)));
                                    ui.label(&format!(
                                        "{} (ready in {}m {}s)",
                                        memo.title, minutes, seconds
                                    ));
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui.add(egui::Label::new(icons::icon_text(icons::DELAY)));
                                    ui.label(&format!("{} (ready in {}s)", memo.title, seconds));
                                });
                            }
                        }
                    }

                    self.render_memo_item(ui, &memo_clone, false);
                    ui.separator();
                }
            }
        });
    }

    pub fn parse_delay_input(&self) -> Option<u32> {
        if self.delay_input == "00:00" {
            return None;
        }

        let parts: Vec<&str> = self.delay_input.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(hours), Ok(minutes)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if hours < 24 && minutes < 60 {
                    let total_minutes = hours * 60 + minutes;
                    return if total_minutes > 0 {
                        Some(total_minutes)
                    } else {
                        Some(0)
                    };
                }
            }
        }
        None
    }

    pub fn adjust_delay_input(&mut self, delta_minutes: i32) {
        let current_minutes = self.parse_delay_input().unwrap_or(0) as i32;
        let new_minutes = (current_minutes + delta_minutes).max(0) as u32;

        let hours = new_minutes / 60;
        let minutes = new_minutes % 60;
        self.delay_input = format!("{:02}:{:02}", hours, minutes);
    }

    fn parse_memo_text(&self) -> (String, String) {
        let text = self.new_memo_text.trim();
        if let Some(first_newline) = text.find('\n') {
            let title = text[..first_newline].trim().to_string();
            let body = text[first_newline + 1..].trim().to_string();
            (title, body)
        } else {
            (text.to_string(), String::new())
        }
    }

    fn add_parsed_memo(&mut self, delay_minutes: Option<u32>) -> Result<()> {
        let (title, body) = self.parse_memo_text();
        self.add_memo(title, body, delay_minutes)?;
        self.new_memo_text.clear();
        Ok(())
    }

    pub fn get_filtered_memos(
        &self,
        status: MemoStatus,
        search: &str,
    ) -> Vec<(i32, crate::models::MemoData)> {
        let mut memos: Vec<(i32, crate::models::MemoData)> = self
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
