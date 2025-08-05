use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub max_hot_count: usize,
    pub font_family: String,
    pub cold_spotlight_interval_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_hot_count: 10,
            font_family: "monospace".to_string(),
            cold_spotlight_interval_seconds: 300,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MemoStatus {
    Hot,
    Cold,
    Done,
}

impl MemoStatus {
    pub fn to_string(&self) -> &'static str {
        match self {
            MemoStatus::Hot => "hot",
            MemoStatus::Cold => "cold",
            MemoStatus::Done => "done",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "hot" => MemoStatus::Hot,
            "cold" => MemoStatus::Cold,
            "done" => MemoStatus::Done,
            _ => MemoStatus::Hot,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoData {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub status: MemoStatus,
    pub creation_date: DateTime<Utc>,
    pub moved_to_done_date: Option<DateTime<Utc>>,
    pub expanded: bool, // UI state only
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveTab {
    Hot,
    Cold,
    Done,
}
