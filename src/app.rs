use chrono::Utc;
use eframe::egui;
use rand::prelude::IndexedRandom;
use rusqlite::{Connection, Result};
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::database;
use crate::models::{ActiveTab, Config, MemoData, MemoStatus};

pub struct MemoApp {
    db: Connection,
    pub hot_stack: Vec<i32>, // Stack order for hot memos (IDs from top to bottom)
    pub memos: HashMap<i32, MemoData>, // All memo data by ID
    pub new_memo_text: String,
    pub delay_input: String,      // HH:MM format for memo delay
    pub prev_delay_input: String, // Previous delay input for tracking changes
    pub config: Config,

    pub active_tab: ActiveTab,
    pub cold_search: String,
    pub done_search: String,
    pub current_spotlight_memo: Option<i32>,
    last_spotlight_update: Option<Instant>,
    pub always_on_top: bool,
    pub memo_input_height: f32,
    pub window_width: f32,
    pub window_height: f32,
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
}

impl MemoApp {
    pub fn new() -> Result<Self> {
        // Get proper data directory
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("memo-stack");

        // Create directory if it doesn't exist
        fs::create_dir_all(&data_dir).unwrap_or_else(|_| {
            eprintln!("Warning: Could not create data directory, using current directory");
        });

        let db_path = data_dir.join("memos.db");
        let config_path = data_dir.join("config.yaml");
        let db = Connection::open(&db_path)?;

        // Load or create config
        let config = Self::load_config(&config_path);

        // Create tables
        database::create_tables(&db)?;

        let mut app = Self {
            db,
            hot_stack: Vec::new(),
            memos: HashMap::new(),
            new_memo_text: String::new(),
            delay_input: String::from("00:00"),
            prev_delay_input: String::from("00:00"),
            config,
            active_tab: ActiveTab::Hot,
            cold_search: String::new(),
            done_search: String::new(),
            current_spotlight_memo: None,
            last_spotlight_update: None,
            always_on_top: false,
            memo_input_height: 80.0,
            window_width: 800.0,
            window_height: 600.0,
            window_x: None,
            window_y: None,
        };

        app.load_state()?;
        Ok(app)
    }

    fn load_config(config_path: &PathBuf) -> Config {
        if config_path.exists() {
            match fs::read_to_string(config_path) {
                Ok(content) => match serde_yaml::from_str(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Error parsing config file: {}, using defaults", e);
                        let default_config = Config::default();
                        Self::save_config(config_path, &default_config);
                        default_config
                    }
                },
                Err(e) => {
                    eprintln!("Error reading config file: {}, using defaults", e);
                    let default_config = Config::default();
                    Self::save_config(config_path, &default_config);
                    default_config
                }
            }
        } else {
            let default_config = Config::default();
            Self::save_config(config_path, &default_config);
            default_config
        }
    }

    fn save_config(config_path: &PathBuf, config: &Config) {
        match serde_yaml::to_string(config) {
            Ok(yaml) => {
                if let Err(e) = fs::write(config_path, yaml) {
                    eprintln!("Error writing config file: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error serializing config: {}", e);
            }
        }
    }

    fn load_state(&mut self) -> Result<()> {
        let (hot_stack, memos) = database::load_state(&self.db)?;
        self.hot_stack = hot_stack;
        self.memos = memos;
        database::save_hot_stack(&self.db, &self.hot_stack)?;

        // Load app state
        let (
            memo_input_height,
            always_on_top,
            new_memo_text,
            window_width,
            window_height,
            window_x,
            window_y,
        ) = database::load_app_state(&self.db)?;
        self.memo_input_height = memo_input_height;
        self.always_on_top = always_on_top;
        self.new_memo_text = new_memo_text;
        self.window_width = window_width;
        self.window_height = window_height;
        self.window_x = window_x;
        self.window_y = window_y;

        Ok(())
    }

    pub fn save_app_state(&self) -> Result<()> {
        database::save_app_state(
            &self.db,
            self.memo_input_height,
            self.always_on_top,
            &self.new_memo_text,
            self.window_width,
            self.window_height,
            self.window_x,
            self.window_y,
        )
    }

    pub fn add_memo(
        &mut self,
        title: String,
        body: String,
        delay_minutes: Option<u32>,
    ) -> Result<()> {
        let new_id = database::add_memo(&self.db, &title, &body, delay_minutes)?;

        // Add to memos map
        let now = Utc::now();
        let status = if delay_minutes.is_some() {
            MemoStatus::Delayed
        } else {
            MemoStatus::Hot
        };

        self.memos.insert(
            new_id,
            MemoData {
                id: new_id,
                title,
                body,
                status,
                creation_date: now,
                moved_to_done_date: None,
                delay_minutes,
                expanded: false,
            },
        );

        // Only add to hot stack if it's not delayed
        if status == MemoStatus::Hot {
            // Add to front of hot stack
            self.hot_stack.insert(0, new_id);

            // If hot stack is too big, move the last item to cold
            if self.hot_stack.len() > self.config.max_hot_count {
                if let Some(moved_id) = self.hot_stack.pop() {
                    self.move_to_cold(moved_id)?;
                }
            }

            database::save_hot_stack(&self.db, &self.hot_stack)?;
        }
        Ok(())
    }

    pub fn move_to_cold(&mut self, id: i32) -> Result<()> {
        if let Some(memo) = self.memos.get_mut(&id) {
            memo.status = MemoStatus::Cold;
            database::update_memo_status(&self.db, id, MemoStatus::Cold)?;
        }
        self.hot_stack.retain(|&x| x != id);
        database::save_hot_stack(&self.db, &self.hot_stack)?;
        Ok(())
    }

    pub fn move_to_done(&mut self, id: i32) -> Result<()> {
        if let Some(memo) = self.memos.get_mut(&id) {
            let now = Utc::now();
            memo.status = MemoStatus::Done;
            memo.moved_to_done_date = Some(now);
            database::update_memo_status(&self.db, id, MemoStatus::Done)?;
        }
        self.hot_stack.retain(|&x| x != id);
        database::save_hot_stack(&self.db, &self.hot_stack)?;
        Ok(())
    }

    pub fn move_to_hot(&mut self, id: i32) -> Result<()> {
        if let Some(memo) = self.memos.get_mut(&id) {
            memo.status = MemoStatus::Hot;
            memo.moved_to_done_date = None;
            database::update_memo_status(&self.db, id, MemoStatus::Hot)?;

            // Add to front of hot stack
            self.hot_stack.insert(0, id);

            // If hot stack is too big, move the last item to cold
            if self.hot_stack.len() > self.config.max_hot_count {
                if let Some(moved_id) = self.hot_stack.pop() {
                    self.move_to_cold(moved_id)?;
                }
            }

            database::save_hot_stack(&self.db, &self.hot_stack)?;
        }
        Ok(())
    }

    pub fn delete_memo(&mut self, id: i32) -> Result<()> {
        // Remove from database
        database::delete_memo(&self.db, id)?;

        // Remove from memory
        self.memos.remove(&id);
        self.hot_stack.retain(|&x| x != id);

        database::save_hot_stack(&self.db, &self.hot_stack)?;
        Ok(())
    }

    pub fn shift_up_in_hot(&mut self, id: i32) -> Result<()> {
        if let Some(pos) = self.hot_stack.iter().position(|&x| x == id) {
            if pos > 0 {
                self.hot_stack.swap(pos - 1, pos);
                database::save_hot_stack(&self.db, &self.hot_stack)?;
            }
        }
        Ok(())
    }

    pub fn move_to_top_in_hot(&mut self, id: i32) -> Result<()> {
        // Remove from current position
        self.hot_stack.retain(|&x| x != id);

        // Add to front
        self.hot_stack.insert(0, id);

        database::save_hot_stack(&self.db, &self.hot_stack)?;
        Ok(())
    }

    pub fn replace_memo(&mut self, id: i32) -> Result<()> {
        // If there's existing text in the input area, save it as a memo first
        if !self.new_memo_text.trim().is_empty() {
            let (title, body) = self.parse_memo_text();
            self.add_memo(title, body, None)?;
        }

        if let Some(memo) = self.memos.get(&id) {
            // Format text for input field
            self.new_memo_text = if memo.body.is_empty() {
                memo.title.clone()
            } else {
                format!("{}\n{}", memo.title, memo.body)
            };

            // Delete the original memo
            self.delete_memo(id)?;
        }
        Ok(())
    }

    pub fn update_cold_spotlight(&mut self) {
        if self.config.cold_spotlight_interval_seconds == 0 {
            return;
        }

        let now = Instant::now();
        let should_update = match self.last_spotlight_update {
            None => true,
            Some(last_update) => {
                now.duration_since(last_update).as_secs()
                    >= self.config.cold_spotlight_interval_seconds
            }
        };

        if should_update {
            self.current_spotlight_memo = self.get_random_cold_memo_id();
            self.last_spotlight_update = Some(now);
        }
    }

    pub fn get_last_spotlight_update(&self) -> Option<std::time::Instant> {
        self.last_spotlight_update
    }

    fn get_random_cold_memo_id(&self) -> Option<i32> {
        let cold_memo_ids: Vec<i32> = self
            .memos
            .iter()
            .filter(|(_, memo)| memo.status == MemoStatus::Cold)
            .map(|(&id, _)| id)
            .collect();

        cold_memo_ids.choose(&mut rand::rng()).copied()
    }

    pub fn check_and_promote_delayed_memos(&mut self) -> Result<()> {
        let now = Utc::now();
        let mut to_promote = Vec::new();

        // Find delayed memos that are ready to be promoted
        for (id, memo) in &self.memos {
            if memo.status == MemoStatus::Delayed {
                if let Some(delay_minutes) = memo.delay_minutes {
                    let promotion_time =
                        memo.creation_date + chrono::Duration::minutes(delay_minutes as i64);

                    if now >= promotion_time {
                        to_promote.push(*id);
                    }
                }
            }
        }

        // Promote memos to hot
        for id in to_promote {
            self.move_to_hot(id)?;
        }

        Ok(())
    }

    // Helper method to indent or outdent selected lines
    // Helper method to indent or outdent selected lines - simplified approach
    pub fn handle_tab_indent(&mut self, cursor_pos: usize, is_indent: bool) {
        let tab_string = " ".repeat(self.config.tab_spaces);

        // Find start of current line
        let line_start = self.new_memo_text[..cursor_pos]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        if is_indent {
            // Insert spaces at beginning of current line
            self.new_memo_text.insert_str(line_start, &tab_string);
        } else {
            // Try to remove spaces from beginning of current line
            let line_end = self.new_memo_text[cursor_pos..]
                .find('\n')
                .map(|pos| cursor_pos + pos)
                .unwrap_or(self.new_memo_text.len());

            let line = &self.new_memo_text[line_start..line_end];

            // Remove up to tab_spaces spaces from beginning
            for spaces_to_remove in (1..=self.config.tab_spaces).rev() {
                let spaces = " ".repeat(spaces_to_remove);
                if line.starts_with(&spaces) {
                    self.new_memo_text
                        .replace_range(line_start..line_start + spaces_to_remove, "");
                    break;
                }
            }
        }
    }

    // Helper method to insert tab spaces at cursor position
    pub fn handle_tab_insert(
        &mut self,
        cursor_pos: usize,
        ui: &mut egui::Ui,
        text_edit_id: egui::Id,
    ) {
        let tab_string = " ".repeat(self.config.tab_spaces);
        self.new_memo_text.insert_str(cursor_pos, &tab_string);

        // Move cursor to after the inserted spaces
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id) {
            let new_cursor_pos = cursor_pos + self.config.tab_spaces;
            let ccursor = egui::text::CCursor::new(new_cursor_pos);
            state.cursor = egui::text_selection::TextCursorState::default();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
            state.store(ui.ctx(), text_edit_id);
            ui.ctx().memory_mut(|mem| mem.request_focus(text_edit_id));
        }
    }

    // Helper method to indent or outdent multiple lines in a selection
    pub fn handle_multiline_indent(
        &mut self,
        start_pos: usize,
        end_pos: usize,
        is_indent: bool,
        ui: &mut egui::Ui,
        text_edit_id: egui::Id,
    ) {
        let tab_string = " ".repeat(self.config.tab_spaces);

        // Find the start of the first selected line
        let line_start = self.new_memo_text[..start_pos]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        // Find the end of the last selected line
        let line_end = if end_pos == self.new_memo_text.len() {
            end_pos
        } else {
            self.new_memo_text[end_pos..]
                .find('\n')
                .map(|pos| end_pos + pos)
                .unwrap_or(self.new_memo_text.len())
        };

        let selected_lines_text = &self.new_memo_text[line_start..line_end];
        let lines: Vec<&str> = selected_lines_text.lines().collect();

        let mut new_text = String::new();

        for (i, line) in lines.iter().enumerate() {
            if is_indent {
                // Add indentation
                new_text.push_str(&tab_string);
                new_text.push_str(line);
            } else {
                // Remove indentation - try to remove up to tab_spaces spaces from beginning
                let mut removed = false;
                for spaces_to_remove in (1..=self.config.tab_spaces).rev() {
                    let spaces = " ".repeat(spaces_to_remove);
                    if line.starts_with(&spaces) {
                        new_text.push_str(&line[spaces_to_remove..]);
                        removed = true;
                        break;
                    }
                }
                if !removed {
                    new_text.push_str(line);
                }
            }

            // Add newline except for the last line if it didn't originally have one
            if i < lines.len() - 1 {
                new_text.push('\n');
            }
        }

        // Calculate new selection positions BEFORE modifying text
        let mut new_start_pos = start_pos;
        let mut new_end_pos = end_pos;
        let lines_count = lines.len();

        if is_indent {
            // For indent: start moves forward by one tab_spaces (for its line)
            // end moves forward by tab_spaces * number of lines
            new_start_pos += self.config.tab_spaces;
            new_end_pos += self.config.tab_spaces * lines_count;
        } else {
            // For outdent: calculate actual spaces removed
            let mut spaces_removed_before_start = 0;
            let mut total_spaces_removed = 0;

            // Count spaces removed for each line
            for (i, line) in lines.iter().enumerate() {
                let mut line_spaces_removed = 0;
                for spaces_to_remove in (1..=self.config.tab_spaces).rev() {
                    let spaces = " ".repeat(spaces_to_remove);
                    if line.starts_with(&spaces) {
                        line_spaces_removed = spaces_to_remove;
                        break;
                    }
                }

                total_spaces_removed += line_spaces_removed;

                // Count spaces removed before start position
                let line_pos_in_selection =
                    line_start + lines.iter().take(i).map(|l| l.len() + 1).sum::<usize>();
                if line_pos_in_selection < start_pos {
                    spaces_removed_before_start += line_spaces_removed;
                }
            }

            new_start_pos = new_start_pos.saturating_sub(spaces_removed_before_start);
            new_end_pos = new_end_pos.saturating_sub(total_spaces_removed);
        }

        // Replace the text (drop all borrows first)
        self.new_memo_text
            .replace_range(line_start..line_end, &new_text);
        // Update the selection range
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id) {
            let start_ccursor = egui::text::CCursor::new(new_start_pos);
            let end_ccursor = egui::text::CCursor::new(new_end_pos);
            state.cursor = egui::text_selection::TextCursorState::default();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    start_ccursor,
                    end_ccursor,
                )));
            state.store(ui.ctx(), text_edit_id);
            ui.ctx().memory_mut(|mem| mem.request_focus(text_edit_id));
        }
    }
}

impl eframe::App for MemoApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Request repaint after 1 second to ensure continuous updates
        ctx.request_repaint_after(Duration::from_millis(500));

        // Track window position and size changes
        let mut window_changed = false;

        ctx.input(|i| {
            // Check window size
            if let Some(inner_rect) = i.viewport().inner_rect {
                let new_width = inner_rect.width();
                let new_height = inner_rect.height();
                if (self.window_width - new_width).abs() > 1.0
                    || (self.window_height - new_height).abs() > 1.0
                {
                    self.window_width = new_width;
                    self.window_height = new_height;
                    window_changed = true;
                }
            }

            // Check window position
            if let Some(outer_rect) = i.viewport().outer_rect {
                let new_x = Some(outer_rect.min.x);
                let new_y = Some(outer_rect.min.y);
                if self.window_x != new_x || self.window_y != new_y {
                    self.window_x = new_x;
                    self.window_y = new_y;
                    window_changed = true;
                }
            }
        });

        // Save app state if window changed
        if window_changed {
            let _ = self.save_app_state();
        }

        // Check for delayed memos that should be promoted
        if let Err(e) = self.check_and_promote_delayed_memos() {
            eprintln!("Error promoting delayed memos: {}", e);
        }

        self.render_ui(ctx, frame);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save app state on shutdown
        let _ = self.save_app_state();
    }
}
