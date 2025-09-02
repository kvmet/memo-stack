use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result};
use std::collections::HashMap;

use crate::models::{MemoData, MemoStatus};

pub fn create_tables(db: &Connection) -> Result<()> {
    // Create tables
    db.execute(
        "CREATE TABLE IF NOT EXISTS memos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'hot',
            creation_date TEXT NOT NULL,
            moved_to_done_date TEXT,
            delay_minutes INTEGER
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

    // Create app_state table
    db.execute(
        "CREATE TABLE IF NOT EXISTS app_state (
            id INTEGER PRIMARY KEY DEFAULT 1,
            memo_input_height REAL NOT NULL DEFAULT 180.0,
            always_on_top INTEGER NOT NULL DEFAULT 0,
            new_memo_text TEXT NOT NULL DEFAULT '',
            window_width REAL NOT NULL DEFAULT 800.0,
            window_height REAL NOT NULL DEFAULT 600.0,
            window_x REAL,
            window_y REAL
        )",
        [],
    )?;

    db.execute("INSERT OR IGNORE INTO app_state (id) VALUES (1)", [])?;

    // Add delay_minutes column if it doesn't exist (migration)
    let _ = db.execute("ALTER TABLE memos ADD COLUMN delay_minutes INTEGER", []);

    // Add window position/size columns if they don't exist (migration)
    let _ = db.execute(
        "ALTER TABLE app_state ADD COLUMN window_width REAL NOT NULL DEFAULT 800.0",
        [],
    );
    let _ = db.execute(
        "ALTER TABLE app_state ADD COLUMN window_height REAL NOT NULL DEFAULT 600.0",
        [],
    );
    let _ = db.execute("ALTER TABLE app_state ADD COLUMN window_x REAL", []);
    let _ = db.execute("ALTER TABLE app_state ADD COLUMN window_y REAL", []);

    Ok(())
}

pub fn load_state(db: &Connection) -> Result<(Vec<i32>, HashMap<i32, MemoData>)> {
    // Load hot stack order
    let stack_json: String = db.query_row(
        "SELECT stack_json FROM hot_stack_state WHERE id = 1",
        [],
        |row| row.get(0),
    )?;

    let mut hot_stack: Vec<i32> = serde_json::from_str(&stack_json).unwrap_or_default();

    // Load all memos
    let mut memos = HashMap::new();
    let mut stmt =
        db.prepare("SELECT id, title, body, status, creation_date, moved_to_done_date, delay_minutes FROM memos")?;
    let memo_iter = stmt.query_map([], |row| {
        let id: i32 = row.get(0)?;
        let creation_date_str: String = row.get(4)?;
        let moved_to_done_date_str: Option<String> = row.get(5)?;
        let delay_minutes: Option<u32> = row.get::<_, Option<i32>>(6)?.map(|v| v as u32);

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
                delay_minutes,
                expanded: false,
            },
        ))
    })?;

    for memo_result in memo_iter {
        let (id, memo) = memo_result?;
        memos.insert(id, memo);
    }

    // Clean up hot stack - remove any IDs that don't exist or aren't hot
    hot_stack.retain(|id| {
        memos
            .get(id)
            .map_or(false, |memo| memo.status == MemoStatus::Hot)
    });

    Ok((hot_stack, memos))
}

pub fn save_hot_stack(db: &Connection, hot_stack: &[i32]) -> Result<()> {
    let stack_json = serde_json::to_string(hot_stack).unwrap();
    db.execute(
        "UPDATE hot_stack_state SET stack_json = ?1 WHERE id = 1",
        [stack_json],
    )?;
    Ok(())
}

pub fn add_memo(
    db: &Connection,
    title: &str,
    body: &str,
    delay_minutes: Option<u32>,
) -> Result<i32> {
    let now = Utc::now();

    // Insert memo into database
    let status = if delay_minutes.is_some() {
        "delayed"
    } else {
        "hot"
    };
    let delay_value = delay_minutes.map(|v| v as i32);

    db.execute(
        "INSERT INTO memos (title, body, status, creation_date, delay_minutes) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![title, body, status, &now.to_rfc3339(), delay_value],
    )?;

    // Get the new memo ID
    let new_id = db.last_insert_rowid() as i32;
    Ok(new_id)
}

pub fn update_memo_status(db: &Connection, id: i32, status: MemoStatus) -> Result<()> {
    match status {
        MemoStatus::Done => {
            let now = Utc::now();
            db.execute(
                "UPDATE memos SET status = 'done', moved_to_done_date = ?1 WHERE id = ?2",
                [&now.to_rfc3339(), &id.to_string()],
            )?;
        }
        MemoStatus::Hot => {
            db.execute(
                "UPDATE memos SET status = 'hot', moved_to_done_date = NULL WHERE id = ?1",
                [id],
            )?;
        }
        MemoStatus::Cold => {
            db.execute("UPDATE memos SET status = 'cold' WHERE id = ?1", [id])?;
        }
        MemoStatus::Delayed => {
            db.execute("UPDATE memos SET status = 'delayed' WHERE id = ?1", [id])?;
        }
    }
    Ok(())
}

pub fn delete_memo(db: &Connection, id: i32) -> Result<()> {
    db.execute("DELETE FROM memos WHERE id = ?1", [id])?;
    Ok(())
}

pub fn load_app_state(
    db: &Connection,
) -> Result<(f32, bool, String, f32, f32, Option<f32>, Option<f32>)> {
    let result = db.query_row(
        "SELECT memo_input_height, always_on_top, new_memo_text, window_width, window_height, window_x, window_y FROM app_state WHERE id = 1",
        [],
        |row| {
            Ok((
                row.get::<_, f64>(0)? as f32, // memo_input_height
                row.get::<_, i32>(1)? != 0,   // always_on_top
                row.get::<_, String>(2)?,     // new_memo_text
                row.get::<_, f64>(3)? as f32, // window_width
                row.get::<_, f64>(4)? as f32, // window_height
                row.get::<_, Option<f64>>(5)?.map(|x| x as f32), // window_x
                row.get::<_, Option<f64>>(6)?.map(|y| y as f32), // window_y
            ))
        },
    )?;
    Ok(result)
}

pub fn load_window_state() -> Result<(f32, f32, Option<f32>, Option<f32>)> {
    use std::path::PathBuf;

    let data_dir = match dirs::data_dir() {
        Some(mut path) => {
            path.push("memo-stack");
            std::fs::create_dir_all(&path).unwrap_or(());
            path
        }
        None => PathBuf::from("."),
    };

    let db_path = data_dir.join("memos.db");
    let db = Connection::open(&db_path)?;
    create_tables(&db)?;

    let (_, _, _, window_width, window_height, window_x, window_y) = load_app_state(&db)?;
    Ok((window_width, window_height, window_x, window_y))
}

pub fn save_app_state(
    db: &Connection,
    memo_input_height: f32,
    always_on_top: bool,
    new_memo_text: &str,
    window_width: f32,
    window_height: f32,
    window_x: Option<f32>,
    window_y: Option<f32>,
) -> Result<()> {
    db.execute(
        "UPDATE app_state SET memo_input_height = ?1, always_on_top = ?2, new_memo_text = ?3, window_width = ?4, window_height = ?5, window_x = ?6, window_y = ?7 WHERE id = 1",
        rusqlite::params![
            memo_input_height as f64,
            if always_on_top { 1 } else { 0 },
            new_memo_text,
            window_width as f64,
            window_height as f64,
            window_x.map(|x| x as f64),
            window_y.map(|y| y as f64)
        ],
    )?;
    Ok(())
}
