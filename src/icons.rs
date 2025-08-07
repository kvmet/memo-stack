// Functional icon constants for the memo app
// Using Phosphor Icons

pub const ADD: &str = "\u{e913}"; // Plus icon for adding memos
pub const DELETE: &str = "\u{E4A8}"; // Trash icon for deleting memos
pub const EDIT: &str = "\u{e96f}"; // Pencil icon for editing/replacing memos
pub const DONE: &str = "\u{e937}"; // Check icon for completed memos
pub const HOT: &str = "\u{E620}"; // Fire icon for hot memos
pub const COLD: &str = "\u{E5AA}"; // Snowflake icon for cold memos
pub const MOVE_UP: &str = "\u{E13C}"; // Arrow up for moving memo up in stack
pub const MOVE_TO_TOP: &str = "\u{E12E}"; // Arrow square up for moving to top
pub const SEARCH: &str = "\u{E30C}"; // Magnifying glass for search
pub const SETTINGS: &str = "\u{e959}"; // Gear icon for settings
pub const DELAY: &str = "\u{E492}"; // Clock icon for delayed memos
pub const ARCHIVE: &str = "\u{e928}"; // Archive icon for cold storage
pub const SPOTLIGHT: &str = "\u{e952}"; // Eye icon for spotlight feature
pub const EXPAND: &str = "\u{E13A}"; // Caret up for expand
pub const COLLAPSE: &str = "\u{E136}"; // Caret down for collapse
pub const CLOSE: &str = "\u{e9b1}"; // X icon for closing/canceling

// Helper function to render an icon with default size
pub fn icon_text(icon: &str) -> egui::RichText {
    egui::RichText::new(icon).font(egui::FontId::new(
        16.0,
        egui::FontFamily::Name("phosphor_icons".into()),
    ))
}

// Helper function to render an icon with custom size
pub fn icon_sized(icon: &str, size: f32) -> egui::RichText {
    egui::RichText::new(icon).font(egui::FontId::new(
        size,
        egui::FontFamily::Name("phosphor_icons".into()),
    ))
}

// Helper function to render an icon with text
pub fn icon_with_text(icon: &str, text: &str) -> String {
    format!("{} {}", icon, text)
}
