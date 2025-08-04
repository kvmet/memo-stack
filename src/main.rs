use eframe::egui;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    max_memo_count: usize,
    font_family: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_memo_count: 20,
            font_family: "monospace".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct MemoData {
    id: i32,
    title: String,
    body: String,
    expanded: bool, // UI state only
}

struct MemoApp {
    db: Connection,
    stack: Vec<i32>,               // Stack order (IDs from top to bottom)
    memos: HashMap<i32, MemoData>, // All memo data by ID
    new_memo_text: String,
    config: Config,
    config_path: PathBuf,
}

impl MemoApp {
    fn new() -> Result<Self> {
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
        db.execute(
            "CREATE TABLE IF NOT EXISTS memos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                body TEXT NOT NULL
            )",
            [],
        )?;

        db.execute(
            "CREATE TABLE IF NOT EXISTS stack_state (
                id INTEGER PRIMARY KEY DEFAULT 1,
                stack_json TEXT NOT NULL DEFAULT '[]'
            )",
            [],
        )?;

        // Initialize stack_state if empty
        db.execute(
            "INSERT OR IGNORE INTO stack_state (id, stack_json) VALUES (1, '[]')",
            [],
        )?;

        let mut app = Self {
            db,
            stack: Vec::new(),
            memos: HashMap::new(),
            new_memo_text: String::new(),
            config,
            config_path,
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
        // Load stack order
        let stack_json: String = self.db.query_row(
            "SELECT stack_json FROM stack_state WHERE id = 1",
            [],
            |row| row.get(0),
        )?;

        self.stack = serde_json::from_str(&stack_json).unwrap_or_default();

        // Load all memos
        self.memos.clear();
        let mut stmt = self.db.prepare("SELECT id, title, body FROM memos")?;
        let memo_iter = stmt.query_map([], |row| {
            let id: i32 = row.get(0)?;
            Ok((
                id,
                MemoData {
                    id,
                    title: row.get(1)?,
                    body: row.get(2)?,
                    expanded: false,
                },
            ))
        })?;

        for memo_result in memo_iter {
            let (id, memo) = memo_result?;
            self.memos.insert(id, memo);
        }

        // Clean up stack - remove any IDs that don't exist in memos
        self.stack.retain(|id| self.memos.contains_key(id));
        self.save_stack()?;

        Ok(())
    }

    fn save_stack(&self) -> Result<()> {
        let stack_json = serde_json::to_string(&self.stack).unwrap();
        self.db.execute(
            "UPDATE stack_state SET stack_json = ?1 WHERE id = 1",
            [stack_json],
        )?;
        Ok(())
    }

    fn add_memo(&mut self, title: String, body: String) -> Result<()> {
        // Insert memo into database
        self.db.execute(
            "INSERT INTO memos (title, body) VALUES (?1, ?2)",
            [&title, &body],
        )?;

        // Get the new memo ID
        let new_id = self.db.last_insert_rowid() as i32;

        // Add to memos map
        self.memos.insert(
            new_id,
            MemoData {
                id: new_id,
                title,
                body,
                expanded: false,
            },
        );

        // Add to front of stack
        self.stack.insert(0, new_id);

        // If stack is too big, remove the last item
        if self.stack.len() > self.config.max_memo_count {
            if let Some(removed_id) = self.stack.pop() {
                self.delete_memo_by_id(removed_id)?;
            }
        }

        self.save_stack()?;
        Ok(())
    }

    fn delete_memo(&mut self, id: i32) -> Result<()> {
        self.delete_memo_by_id(id)?;
        self.load_state()?; // Reload to ensure consistency
        Ok(())
    }

    fn delete_memo_by_id(&mut self, id: i32) -> Result<()> {
        // Remove from database
        self.db.execute("DELETE FROM memos WHERE id = ?1", [id])?;

        // Remove from memory
        self.memos.remove(&id);
        self.stack.retain(|&x| x != id);

        self.save_stack()?;
        Ok(())
    }

    fn shift_up(&mut self, id: i32) -> Result<()> {
        if let Some(pos) = self.stack.iter().position(|&x| x == id) {
            if pos > 0 {
                self.stack.swap(pos - 1, pos);
                self.save_stack()?;
            }
        }
        Ok(())
    }

    fn move_to_top(&mut self, id: i32) -> Result<()> {
        // Remove from current position
        self.stack.retain(|&x| x != id);

        // Add to front
        self.stack.insert(0, id);

        self.save_stack()?;
        Ok(())
    }

    fn replace_memo(&mut self, id: i32) -> Result<()> {
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

    fn get_font_family(&self) -> egui::FontFamily {
        match self.config.font_family.as_str() {
            "monospace" => egui::FontFamily::Monospace,
            _ => egui::FontFamily::Proportional,
        }
    }
}

impl eframe::App for MemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Memo Stack");
            ui.separator();

            // Input section
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

            // Display stack info
            ui.label(format!(
                "Memos: {}/{}",
                self.stack.len(),
                self.config.max_memo_count
            ));

            // Display memos in stack order
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Collect memo data before UI loop to avoid borrow checker issues
                let memo_data: Vec<(usize, MemoData)> = self
                    .stack
                    .iter()
                    .enumerate()
                    .filter_map(|(pos, id)| self.memos.get(id).map(|memo| (pos, memo.clone())))
                    .collect();

                for (stack_pos, memo) in memo_data {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Shift up button (not for top item)
                            if stack_pos > 0 {
                                let shift_pressed = ui.input(|i| i.modifiers.shift);
                                let button_text = if shift_pressed { "^^" } else { "^" };
                                if ui.button(button_text).clicked() {
                                    if shift_pressed {
                                        if let Err(e) = self.move_to_top(memo.id) {
                                            eprintln!("Error moving to top: {}", e);
                                        }
                                    } else {
                                        if let Err(e) = self.shift_up(memo.id) {
                                            eprintln!("Error shifting memo: {}", e);
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

                            // Replace button
                            if ui.button("⟲").clicked() {
                                if let Err(e) = self.replace_memo(memo.id) {
                                    eprintln!("Error replacing memo: {}", e);
                                }
                            }

                            // Delete button
                            if ui.button("✕").clicked() {
                                if let Err(e) = self.delete_memo(memo.id) {
                                    eprintln!("Error deleting memo: {}", e);
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
                    });
                }
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let app = MemoApp::new().expect("Failed to initialize app");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native("Memo Stack", options, Box::new(|_cc| Ok(Box::new(app))))
}
