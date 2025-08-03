use eframe::egui;
use rusqlite::{Connection, Result};

const MAX_MEMO_COUNT: usize = 20;

#[derive(Debug, Clone)]
struct MemoItem {
    id: i32,
    title: String,
    body: String,
    position: i32,
    expanded: bool, // UI state, not persisted
}

impl MemoItem {
    fn new(id: i32, title: String, body: String, position: i32) -> Self {
        Self {
            id,
            title,
            body,
            position,
            expanded: false,
        }
    }
}

struct MemoApp {
    db: Connection,
    memos: Vec<MemoItem>,
    new_memo_text: String,
}

impl MemoApp {
    fn new() -> Result<Self> {
        let db = Connection::open("memos.db")?;

        // Create table if it doesn't exist
        db.execute(
            "CREATE TABLE IF NOT EXISTS memos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                position INTEGER NOT NULL
            )",
            [],
        )?;

        let mut app = Self {
            db,
            memos: Vec::new(),
            new_memo_text: String::new(),
        };

        app.load_memos()?;
        Ok(app)
    }

    fn load_memos(&mut self) -> Result<()> {
        let mut stmt = self
            .db
            .prepare("SELECT id, title, body, position FROM memos ORDER BY position ASC")?;

        let memo_iter = stmt.query_map([], |row| {
            Ok(MemoItem::new(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
            ))
        })?;

        self.memos.clear();
        for memo in memo_iter {
            self.memos.push(memo?);
        }

        Ok(())
    }

    fn add_memo(&mut self, title: String, body: String) -> Result<()> {
        // If we're at max capacity, remove the top item (position 0)
        if self.memos.len() >= MAX_MEMO_COUNT {
            self.remove_top_memo()?;
        }

        // Shift all existing memos up
        self.db
            .execute("UPDATE memos SET position = position + 1", [])?;

        // Insert new memo at position 0
        self.db.execute(
            "INSERT INTO memos (title, body, position) VALUES (?1, ?2, 0)",
            [&title, &body],
        )?;

        self.load_memos()?;
        Ok(())
    }

    fn remove_top_memo(&mut self) -> Result<()> {
        self.db.execute(
            "DELETE FROM memos WHERE position = (SELECT MIN(position) FROM memos)",
            [],
        )?;
        Ok(())
    }

    fn delete_memo(&mut self, id: i32) -> Result<()> {
        // Get the position of the memo to delete
        let position: i32 =
            self.db
                .query_row("SELECT position FROM memos WHERE id = ?1", [id], |row| {
                    row.get(0)
                })?;

        // Delete the memo
        self.db.execute("DELETE FROM memos WHERE id = ?1", [id])?;

        // Shift down all memos that were below the deleted one
        self.db.execute(
            "UPDATE memos SET position = position - 1 WHERE position > ?1",
            [position],
        )?;

        self.load_memos()?;
        Ok(())
    }

    fn shift_memo_up(&mut self, id: i32) -> Result<()> {
        let position: i32 =
            self.db
                .query_row("SELECT position FROM memos WHERE id = ?1", [id], |row| {
                    row.get(0)
                })?;

        if position > 0 {
            // Swap positions with the memo above
            self.db.execute(
                "UPDATE memos SET position = position + 1 WHERE position = ?1",
                [position - 1],
            )?;
            self.db.execute(
                "UPDATE memos SET position = ?1 WHERE id = ?2",
                [position - 1, id],
            )?;

            self.load_memos()?;
        }

        Ok(())
    }

    fn replace_memo(&mut self, id: i32) -> Result<()> {
        // Get the memo text to copy to input
        let (title, body): (String, String) =
            self.db
                .query_row("SELECT title, body FROM memos WHERE id = ?1", [id], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?;

        // Format text for input field (title on first line, body after)
        self.new_memo_text = if body.is_empty() {
            title
        } else {
            format!("{}\n{}", title, body)
        };

        // Delete the original memo
        self.delete_memo(id)?;
        Ok(())
    }
}

impl eframe::App for MemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Memo Stack");
            ui.separator();

            // Input section for new memo
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

            // Display memos
            let memo_count = self.memos.len();
            ui.label(format!("Memos: {}/{}", memo_count, MAX_MEMO_COUNT));

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut memos_copy = self.memos.clone();

                for memo in &mut memos_copy {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Shift up button
                            if memo.position > 0 && ui.button("^").clicked() {
                                if let Err(e) = self.shift_memo_up(memo.id) {
                                    eprintln!("Error shifting memo: {}", e);
                                }
                            }

                            // Expand/collapse button (only if there's body text)
                            if !memo.body.is_empty() {
                                let expand_text = if memo.expanded { "−" } else { "+" };
                                if ui.button(expand_text).clicked() {
                                    // Find the memo in the original vector and toggle expanded
                                    if let Some(original_memo) =
                                        self.memos.iter_mut().find(|m| m.id == memo.id)
                                    {
                                        original_memo.expanded = !original_memo.expanded;
                                    }
                                }
                            }

                            // Replace button
                            if ui.button("⟲").clicked() {
                                if let Err(e) = self.replace_memo(memo.id) {
                                    eprintln!("Error replacing memo: {}", e);
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
                            ui.label(&memo.title);
                        });

                        // Show body if expanded and not empty
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
