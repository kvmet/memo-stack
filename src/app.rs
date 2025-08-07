use chrono::Utc;
use eframe::egui;
use rand::prelude::IndexedRandom;
use rusqlite::{Connection, Result};
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::database;
use crate::models::{ActiveTab, Config, MemoData, MemoStatus};

pub struct MemoApp {
    db: Connection,
    pub hot_stack: Vec<i32>, // Stack order for hot memos (IDs from top to bottom)
    pub memos: HashMap<i32, MemoData>, // All memo data by ID
    pub new_memo_text: String,
    pub delay_input: String, // HH:MM format for memo delay
    pub config: Config,
    config_path: PathBuf,
    pub active_tab: ActiveTab,
    pub cold_search: String,
    pub done_search: String,
    pub current_spotlight_memo: Option<i32>,
    last_spotlight_update: Option<Instant>,
    pub always_on_top: bool,
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
            config,
            config_path,
            active_tab: ActiveTab::Hot,
            cold_search: String::new(),
            done_search: String::new(),
            current_spotlight_memo: None,
            last_spotlight_update: None,
            always_on_top: true,
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
        Ok(())
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

    fn get_random_cold_memo_id(&self) -> Option<i32> {
        let cold_memo_ids: Vec<i32> = self
            .memos
            .iter()
            .filter(|(_, memo)| memo.status == MemoStatus::Cold)
            .map(|(&id, _)| id)
            .collect();

        cold_memo_ids.choose(&mut rand::rng()).copied()
    }
}

impl eframe::App for MemoApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.render_ui(ctx, frame);
    }
}
