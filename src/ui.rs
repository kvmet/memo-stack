use crate::app::MemoApp;
use crate::models::{ActiveTab, MemoData, MemoStatus};
use chrono::Utc;
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
                ui.selectable_value(&mut self.active_tab, ActiveTab::Delayed, "⏱ Delayed");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    use std::sync::Once;
                    static INIT: Once = Once::new();

                    let checkbox_changed = ui
                        .checkbox(&mut self.always_on_top, "📌")
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
                let input_max_height = 180.0;
                // Calculate current text height to determine if we need scrolling
                let line_count = self.new_memo_text.lines().count().max(1);
                let text_height = (line_count as f32 * 14.0 + 20.0).min(input_max_height - 30.0);

                egui::ScrollArea::vertical()
                    .max_height(input_max_height - 30.0)
                    .show(ui, |ui| {
                        let text_edit_response = ui.add_sized(
                            [ui.available_width(), text_height],
                            egui::TextEdit::multiline(&mut self.new_memo_text)
                                .hint_text("Type your memo here...\nFirst line becomes the title")
                                .desired_width(f32::INFINITY),
                        );

                        // Handle Tab key to insert 4 spaces instead of changing focus
                        if text_edit_response.has_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Tab))
                        {
                            // Simple approach: add 4 spaces at the end for now
                            self.new_memo_text.push_str("    ");
                        }
                    });

                // Delay input field
                ui.horizontal(|ui| {
                    ui.label("Delay:");
                    let delay_response = ui.add_sized(
                        [60.0, 20.0],
                        egui::TextEdit::singleline(&mut self.delay_input)
                            .hint_text("00:00")
                            .char_limit(5),
                    );

                    ui.label("(HH:MM, Ctrl/Cmd+0-9 minutes, [/] hours)");
                });

                // Handle keyboard shortcuts for delay adjustment (works from any input)
                // Ctrl+0 clears the delay
                if ui.input(|i| {
                    i.key_pressed(egui::Key::Num0) && (i.modifiers.ctrl || i.modifiers.command)
                }) {
                    self.delay_input = String::from("00:00");
                }
                // Ctrl+1-9 adds that many minutes
                for (key, minutes) in [
                    (egui::Key::Num1, 1),
                    (egui::Key::Num2, 2),
                    (egui::Key::Num3, 3),
                    (egui::Key::Num4, 4),
                    (egui::Key::Num5, 5),
                    (egui::Key::Num6, 6),
                    (egui::Key::Num7, 7),
                    (egui::Key::Num8, 8),
                    (egui::Key::Num9, 9),
                    (egui::Key::OpenBracket, -60),
                    (egui::Key::CloseBracket, 60),
                ] {
                    if ui.input(|i| i.key_pressed(key) && (i.modifiers.ctrl || i.modifiers.command))
                    {
                        self.adjust_delay_input(minutes);
                    }
                }

                // Check for any modified enter to submit
                let modified_enter_pressed = ui.input(|i| {
                    i.key_pressed(egui::Key::Enter)
                        && (i.modifiers.shift || i.modifiers.ctrl || i.modifiers.command)
                });

                if ui.button("Add Memo").clicked() || modified_enter_pressed {
                    if !self.new_memo_text.trim().is_empty() {
                        let lines: Vec<&str> = self.new_memo_text.lines().collect();
                        let title = lines.first().unwrap_or(&"").to_string();
                        let body = if lines.len() > 1 {
                            lines[1..].join("\n")
                        } else {
                            String::new()
                        };

                        let delay_minutes = self.parse_delay_input();
                        if let Err(e) = self.add_memo(title, body, delay_minutes) {
                            eprintln!("Error adding memo: {}", e);
                        }
                        self.new_memo_text.clear();
                        self.delay_input = String::from("00:00");
                    }
                }
            });

            ui.add_space(10.0);

            // Section 2: Hot Memos (fills remaining space)
            ui.push_id("hot_memos", |ui| {
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

            // Section 3: Cold Spotlight (sticky at bottom)
            if has_spotlight {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.push_id("cold_spotlight", |ui| {
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

    fn render_delayed_tab(&mut self, ui: &mut egui::Ui) {
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
                            ui.label(format!("🔥 Ready to promote: {}", memo.title));
                        } else {
                            let remaining = promotion_time - now;
                            let total_seconds = remaining.num_seconds();
                            let hours = total_seconds / 3600;
                            let minutes = (total_seconds % 3600) / 60;
                            let seconds = total_seconds % 60;

                            if hours > 0 {
                                ui.label(format!(
                                    "⏱ {} (ready in {}h {}m {}s)",
                                    memo.title, hours, minutes, seconds
                                ));
                            } else if minutes > 0 {
                                ui.label(format!(
                                    "⏱ {} (ready in {}m {}s)",
                                    memo.title, minutes, seconds
                                ));
                            } else {
                                ui.label(format!("⏱ {} (ready in {}s)", memo.title, seconds));
                            }
                        }
                    }

                    self.render_memo_item(ui, &memo_clone, false);
                    ui.separator();
                }
            }
        });
    }

    fn parse_delay_input(&self) -> Option<u32> {
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

    fn adjust_delay_input(&mut self, delta_minutes: i32) {
        let current_minutes = self.parse_delay_input().unwrap_or(0) as i32;
        let new_minutes = (current_minutes + delta_minutes).max(0) as u32;

        let hours = new_minutes / 60;
        let minutes = new_minutes % 60;
        self.delay_input = format!("{:02}:{:02}", hours, minutes);
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
                    MemoStatus::Delayed => {
                        if ui.button("🔥").on_hover_text("Make Hot Now").clicked() {
                            if let Err(e) = self.move_to_hot(memo.id) {
                                eprintln!("Error moving to hot: {}", e);
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
