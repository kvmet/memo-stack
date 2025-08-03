use eframe::egui;
use rusqlite::{Connection, Result};

use std::collections::HashMap;

const MAX_MEMO_COUNT: usize = 20;

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
}

impl MemoApp {
    fn new() -> Result<Self> {
        let db = Connection::open("memos.db")?;

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
        };

        app.load_state()?;
        Ok(app)
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
        if self.stack.len() > MAX_MEMO_COUNT {
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
}

impl eframe::App for MemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Memo Stack");
            ui.separator();

            // Input section
            ui.label("New memo (first line = title):");
            ui.text_edit_multiline(&mut self.new_memo_text);

            if ui.button("Add Memo").clicked() {
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
            ui.label(format!("Memos: {}/{}", self.stack.len(), MAX_MEMO_COUNT));

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
                            if stack_pos > 0 && ui.button("^").clicked() {
                                if let Err(e) = self.shift_up(memo.id) {
                                    eprintln!("Error shifting memo: {}", e);
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

                            // Title (shift+click to move to top)
                            let title_response =
                                ui.add(egui::Label::new(&memo.title).sense(egui::Sense::click()));
                            if title_response.clicked() && ui.input(|i| i.modifiers.shift) {
                                if let Err(e) = self.move_to_top(memo.id) {
                                    eprintln!("Error moving to top: {}", e);
                                }
                            }
                        });

                        // Show body if expanded
                        if memo.expanded && !memo.body.is_empty() {
                            ui.separator();
                            ui.label(&memo.body);
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
