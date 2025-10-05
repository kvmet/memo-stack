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

        // Validate memo input height against available space
        self.validate_memo_input_height(ui.available_height());

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
            // Section 1: Memo Input (manual resizing only)
            ui.push_id("memo_input", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(self.memo_input_height - 30.0)
                    .min_scrolled_height(self.config.memo_input_height_min)
                    .show(ui, |ui| {
                        let output = ui.input_mut(|input| {
                            // Consume Tab keys before TextEdit gets them
                            let shift_tab =
                                input.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
                            let tab = input.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                            (shift_tab, tab)
                        });

                        let text_edit_id = ui.id().with("memo_text_edit");

                        // Simple approach: let TextEdit fill the fixed container
                        let text_edit = egui::TextEdit::multiline(&mut self.new_memo_text)
                            .hint_text("Enter memo...\nCtrl+Enter to add")
                            .desired_width(ui.available_width())
                            .desired_rows(2)
                            .min_size(egui::vec2(0.0, ui.available_height()))
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

                            // Save app state when memo text changes
                            if response.changed() {
                                let _ = self.save_app_state();
                            }
                        }
                    });

                // Add top spacing before buttons row
                ui.add_space(4.0);

                // Buttons row
                ui.horizontal(|ui| {
                    // Add memo button (left aligned)
                    let add_enabled = !self.new_memo_text.trim().is_empty();
                    let delay_minutes = self.parse_delay_input();
                    let button_text = if delay_minutes.is_some() {
                        "Delayed"
                    } else {
                        "Add Hot"
                    };

                    if (icons::button_with_icon(ui, icons::ADD, button_text, add_enabled).clicked()
                        || (ui.input(|i| {
                            i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl && add_enabled
                        })))
                        && add_enabled
                    {
                        if let Err(e) = self.add_parsed_memo(delay_minutes) {
                            eprintln!("Error adding memo: {}", e);
                        }
                    }

                    // Right-aligned delay controls
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Quick delay buttons (in reverse order for right-to-left layout)
                        if ui.small_button("+60").clicked() {
                            self.adjust_delay_input(60);
                        }
                        if ui.small_button("+15").clicked() {
                            self.adjust_delay_input(15);
                        }
                        if ui.small_button("+5").clicked() {
                            self.adjust_delay_input(5);
                        }
                        if ui.small_button("0m").clicked() {
                            self.delay_input = format!("{:02}:{:02}", 0, 0);
                        }

                        // Delay input
                        let delay_response = ui.add_sized(
                            [50.0, 20.0],
                            egui::TextEdit::singleline(&mut self.delay_input).hint_text("HH:MM"),
                        );

                        // Filter input to only allow digits and colons
                        if delay_response.changed() {
                            self.delay_input = self
                                .delay_input
                                .chars()
                                .filter(|c| c.is_ascii_digit() || *c == ':')
                                .collect();
                        }

                        ui.label(icons::icon_text(icons::DELAY))
                            .on_hover_text("Delay (HH:MM)");
                        ui.separator();
                    });
                });
            });

            // Draggable divider for resizing memo input
            ui.push_id("memo_input_divider", |ui| {
                let divider_response = ui
                    .allocate_response(egui::vec2(ui.available_width(), 6.0), egui::Sense::drag());

                // Handle dragging to resize
                if divider_response.dragged() {
                    let new_height = self.memo_input_height + divider_response.drag_delta().y;
                    self.memo_input_height = new_height.clamp(
                        self.config.memo_input_height_min,
                        self.config.memo_input_height_max,
                    );

                    // Save app state to database
                    let _ = self.save_app_state();

                    // Request repaint to apply the change
                    ui.ctx().request_repaint();
                }

                // Draw the divider
                let divider_rect = divider_response.rect;
                let divider_color = if divider_response.hovered() {
                    ui.visuals().widgets.hovered.bg_fill
                } else {
                    ui.visuals().widgets.inactive.bg_fill
                };

                ui.painter()
                    .rect_filled(divider_rect, egui::CornerRadius::same(2), divider_color);

                // Add three dots in the center to indicate it's draggable
                let center = divider_rect.center();
                let dot_color = ui.visuals().text_color();
                let dot_radius = 1.5;
                for i in -1..=1 {
                    let dot_pos = center + egui::vec2(i as f32 * 8.0, 0.0);
                    ui.painter().circle_filled(dot_pos, dot_radius, dot_color);
                }

                // Change cursor when hovering
                if divider_response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }
            });

            // Section 2: Hot Stack (with Cold Spotlight at bottom if available)
            ui.push_id("hot_stack", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        // Render hot memos first
                        for &memo_id in &self.hot_stack.clone() {
                            if let Some(memo) = self.memos.get(&memo_id) {
                                let memo_clone = memo.clone();
                                self.render_memo_item(ui, &memo_clone, true);
                            }
                        }

                        // If we have a spotlight, add it at the bottom
                        if has_spotlight {
                            // Calculate remaining space and add it as spacing to push spotlight down
                            let remaining_height = ui.available_height();
                            if remaining_height > self.config.cold_spotlight_bottom_spacing {
                                // Only add space if there's enough room
                                ui.add_space(
                                    remaining_height - self.config.cold_spotlight_bottom_spacing,
                                );
                            }

                            ui.separator();
                            ui.push_id("cold_spotlight", |ui| {
                                if let Some(spotlight_id) = self.current_spotlight_memo {
                                    if let Some(memo) = self.memos.get(&spotlight_id) {
                                        // Check if spotlight is paused or calculate remaining seconds
                                        let timer_text = if self.is_spotlight_paused() {
                                            "Cold Spotlight: Paused".to_string()
                                        } else {
                                            let remaining_seconds = if let Some(last_update) =
                                                self.get_last_spotlight_update()
                                            {
                                                let elapsed = std::time::Instant::now()
                                                    .duration_since(last_update)
                                                    .as_secs();
                                                self.config
                                                    .cold_spotlight_interval_seconds
                                                    .saturating_sub(elapsed)
                                            } else {
                                                0
                                            };
                                            format!(
                                                "Cold Spotlight: Next in {}s",
                                                remaining_seconds
                                            )
                                        };

                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 4.0;
                                            ui.add(egui::Label::new(icons::icon_text(icons::COLD)));
                                            ui.label(timer_text);
                                        });
                                        let memo_clone = memo.clone();
                                        self.render_memo_item_with_spotlight_state(
                                            ui,
                                            &memo_clone,
                                            false,
                                            true,
                                        );
                                    }
                                }
                            });
                        }
                    });
            });
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
        ui.label(format!(
            "Done memos: {} (Hold shift to delete)",
            done_memos.len()
        ));

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
                if hours < 100 && minutes < 100 {
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

    pub fn parse_memo_text(&self) -> (String, String) {
        let text = self.new_memo_text.trim();
        if let Some(first_newline) = text.find('\n') {
            let title = text[..first_newline].trim().to_string();
            let body = text[first_newline + 1..].trim_end().to_string();
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
