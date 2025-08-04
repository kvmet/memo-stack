use chrono::{DateTime, Utc};
use eframe::egui;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    max_hot_count: usize,
    font_family: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_hot_count: 10,
            font_family: "monospace".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum MemoStatus {
    Hot,
    Cold,
    Done,
}

impl MemoStatus {
    fn to_string(&self) -> &'static str {
        match self {
            MemoStatus::Hot => "hot",
            MemoStatus::Cold => "cold",
            MemoStatus::Done => "done",
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "hot" => MemoStatus::Hot,
            "cold" => MemoStatus::Cold,
            "done" => MemoStatus::Done,
            _ => MemoStatus::Hot,
        }
    }
}

#[derive(Debug, Clone)]
struct MemoData {
    id: i32,
    title: String,
    body: String,
    status: MemoStatus,
    creation_date: DateTime<Utc>,
    moved_to_done_date: Option<DateTime<Utc>>,
    expanded: bool, // UI state only
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ActiveTab {
    Hot,
    Cold,
    Done,
}

struct MemoApp {
    db: Connection,
    hot_stack: Vec<i32>, // Stack order for hot memos (IDs from top to bottom)
    memos: HashMap<i32, MemoData>, // All memo data by ID
    new_memo_text: String,
    config: Config,
    config_path: PathBuf,
    active_tab: ActiveTab,
    cold_search: String,
    done_search: String,
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
        Self::create_tables(&db)?;

        let mut app = Self {
            db,
            hot_stack: Vec::new(),
            memos: HashMap::new(),
            new_memo_text: String::new(),
            config,
            config_path,
            active_tab: ActiveTab::Hot,
            cold_search: String::new(),
            done_search: String::new(),
        };

        app.load_state()?;
        Ok(app)
    }

    fn create_tables(db: &Connection) -> Result<()> {
        // Create tables
        db.execute(
            "CREATE TABLE IF NOT EXISTS memos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'hot',
                creation_date TEXT NOT NULL,
                moved_to_done_date TEXT
            )",
            [],
        )?;

        db.execute(
            "CREATE TABLE IF NOT EXISTS hot_stack_state (
                id INTEGER PRIMARY KEY DEFAULT 1,
                stack_json TEXT NOT NULL DEFAULT '[]'
            )",
            [],
        )?;

        db.execute(
            "INSERT OR IGNORE INTO hot_stack_state (id, stack_json) VALUES (1, '[]')",
            [],
        )?;

        Ok(())
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
        // Load hot stack order
        let stack_json: String = self.db.query_row(
            "SELECT stack_json FROM hot_stack_state WHERE id = 1",
            [],
            |row| row.get(0),
        )?;

        self.hot_stack = serde_json::from_str(&stack_json).unwrap_or_default();

        // Load all memos
        self.memos.clear();
        let mut stmt = self.db.prepare(
            "SELECT id, title, body, status, creation_date, moved_to_done_date FROM memos",
        )?;
        let memo_iter = stmt.query_map([], |row| {
            let id: i32 = row.get(0)?;
            let creation_date_str: String = row.get(4)?;
            let moved_to_done_date_str: Option<String> = row.get(5)?;

            let creation_date = DateTime::parse_from_rfc3339(&creation_date_str)
                .unwrap_or_else(|_| Utc::now().into())
                .with_timezone(&Utc);

            let moved_to_done_date = moved_to_done_date_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            Ok((
                id,
                MemoData {
                    id,
                    title: row.get(1)?,
                    body: row.get(2)?,
                    status: MemoStatus::from_string(&row.get::<_, String>(3)?),
                    creation_date,
                    moved_to_done_date,
                    expanded: false,
                },
            ))
        })?;

        for memo_result in memo_iter {
            let (id, memo) = memo_result?;
            self.memos.insert(id, memo);
        }

        // Clean up hot stack - remove any IDs that don't exist or aren't hot
        self.hot_stack.retain(|id| {
            self.memos
                .get(id)
                .map_or(false, |memo| memo.status == MemoStatus::Hot)
        });
        self.save_hot_stack()?;

        Ok(())
    }

    fn save_hot_stack(&self) -> Result<()> {
        let stack_json = serde_json::to_string(&self.hot_stack).unwrap();
        self.db.execute(
            "UPDATE hot_stack_state SET stack_json = ?1 WHERE id = 1",
            [stack_json],
        )?;
        Ok(())
    }

    fn add_memo(&mut self, title: String, body: String) -> Result<()> {
        let now = Utc::now();

        // Insert memo into database
        self.db.execute(
            "INSERT INTO memos (title, body, status, creation_date) VALUES (?1, ?2, 'hot', ?3)",
            [&title, &body, &now.to_rfc3339()],
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
                status: MemoStatus::Hot,
                creation_date: now,
                moved_to_done_date: None,
                expanded: false,
            },
        );

        // Add to front of hot stack
        self.hot_stack.insert(0, new_id);

        // If hot stack is too big, move the last item to cold
        if self.hot_stack.len() > self.config.max_hot_count {
            if let Some(moved_id) = self.hot_stack.pop() {
                self.move_to_cold(moved_id)?;
            }
        }

        self.save_hot_stack()?;
        Ok(())
    }

    fn move_to_cold(&mut self, id: i32) -> Result<()> {
        if let Some(memo) = self.memos.get_mut(&id) {
            memo.status = MemoStatus::Cold;
            self.db
                .execute("UPDATE memos SET status = 'cold' WHERE id = ?1", [id])?;
        }
        self.hot_stack.retain(|&x| x != id);
        self.save_hot_stack()?;
        Ok(())
    }

    fn move_to_done(&mut self, id: i32) -> Result<()> {
        if let Some(memo) = self.memos.get_mut(&id) {
            let now = Utc::now();
            memo.status = MemoStatus::Done;
            memo.moved_to_done_date = Some(now);
            self.db.execute(
                "UPDATE memos SET status = 'done', moved_to_done_date = ?1 WHERE id = ?2",
                [&now.to_rfc3339(), &id.to_string()],
            )?;
        }
        self.hot_stack.retain(|&x| x != id);
        self.save_hot_stack()?;
        Ok(())
    }

    fn move_to_hot(&mut self, id: i32) -> Result<()> {
        if let Some(memo) = self.memos.get_mut(&id) {
            memo.status = MemoStatus::Hot;
            memo.moved_to_done_date = None;
            self.db.execute(
                "UPDATE memos SET status = 'hot', moved_to_done_date = NULL WHERE id = ?1",
                [id],
            )?;

            // Add to front of hot stack
            self.hot_stack.insert(0, id);

            // If hot stack is too big, move the last item to cold
            if self.hot_stack.len() > self.config.max_hot_count {
                if let Some(moved_id) = self.hot_stack.pop() {
                    self.move_to_cold(moved_id)?;
                }
            }

            self.save_hot_stack()?;
        }
        Ok(())
    }

    fn delete_memo(&mut self, id: i32) -> Result<()> {
        // Remove from database
        self.db.execute("DELETE FROM memos WHERE id = ?1", [id])?;

        // Remove from memory
        self.memos.remove(&id);
        self.hot_stack.retain(|&x| x != id);

        self.save_hot_stack()?;
        Ok(())
    }

    fn shift_up_in_hot(&mut self, id: i32) -> Result<()> {
        if let Some(pos) = self.hot_stack.iter().position(|&x| x == id) {
            if pos > 0 {
                self.hot_stack.swap(pos - 1, pos);
                self.save_hot_stack()?;
            }
        }
        Ok(())
    }

    fn move_to_top_in_hot(&mut self, id: i32) -> Result<()> {
        // Remove from current position
        self.hot_stack.retain(|&x| x != id);

        // Add to front
        self.hot_stack.insert(0, id);

        self.save_hot_stack()?;
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

    fn get_filtered_memos(&self, status: MemoStatus, search: &str) -> Vec<(i32, MemoData)> {
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
}

impl eframe::App for MemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                ActiveTab::Hot => {
                    // Input section (only in Hot tab)
                    ui.label("New memo (first line = title):");
                    let font_id = egui::FontId::new(14.0, self.get_font_family());
                    ui.add(
                        egui::TextEdit::multiline(&mut self.new_memo_text).font(font_id.clone()),
                    );

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

                ActiveTab::Cold => {
                    // Search bar
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cold_search)
                                .hint_text("Search cold memos..."),
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

                ActiveTab::Done => {
                    // Search bar
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.done_search)
                                .hint_text("Search done memos..."),
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
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let app = MemoApp::new().expect("Failed to initialize app");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([500.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native("Memo Stack", options, Box::new(|_cc| Ok(Box::new(app))))
}
