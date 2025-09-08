use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub max_hot_count: usize,
    pub cold_spotlight_interval_seconds: u64,
    pub tab_spaces: usize,
    pub memo_input_height_min: f32,
    pub memo_input_height_max: f32,
    pub cold_spotlight_bottom_spacing: f32,
    pub pause_spotlight_when_expanded: bool,
    pub memo_input_space_buffer: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_hot_count: 7,
            cold_spotlight_interval_seconds: 60,
            tab_spaces: 2,
            memo_input_height_min: 38.0,
            memo_input_height_max: 1200.0,
            cold_spotlight_bottom_spacing: 84.0,
            pause_spotlight_when_expanded: true,
            memo_input_space_buffer: 58.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MemoStatus {
    Hot,
    Cold,
    Done,
    Delayed,
}

impl MemoStatus {
    pub fn from_string(s: &str) -> Self {
        match s {
            "hot" => MemoStatus::Hot,
            "cold" => MemoStatus::Cold,
            "done" => MemoStatus::Done,
            "delayed" => MemoStatus::Delayed,
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
    pub delay_minutes: Option<u32>, // Minutes to delay from creation_date
    pub expanded: bool,             // UI state only
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveTab {
    Hot,
    Cold,
    Done,
    Delayed,
}
