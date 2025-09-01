// Functional icon constants for the memo app
// Using Phosphor Icons

pub const ADD: &str = "\u{E3D6}"; // Plus icon for adding memos
pub const DELETE: &str = "\u{E4F6}"; // Trash icon for deleting memos
pub const EDIT: &str = "\u{E3B4}"; // Pencil icon for editing/replacing memos
pub const DONE: &str = "\u{E182}"; // Check icon for completed memos
pub const HOT: &str = "\u{E620}"; // Fire icon for hot memos
pub const COLD: &str = "\u{E5AA}"; // Snowflake icon for cold memos
pub const MOVE_UP: &str = "\u{E13C}"; // Arrow up for moving memo up in stack
pub const MOVE_TO_TOP: &str = "\u{E12C}"; // Arrow square up for moving to top
pub const SEARCH: &str = "\u{E30C}"; // Magnifying glass for search
pub const DELAY: &str = "\u{E492}"; // Clock icon for delayed memos
pub const EXPAND: &str = "\u{E13A}"; // Caret up for expand
pub const COLLAPSE: &str = "\u{E136}"; // Caret down for collapse
pub const ALWAYS_ON_TOP: &str = "\u{E3E2}"; // X icon for closing/canceling

// Helper function to render an icon with default size
pub fn icon_text(icon: &str) -> egui::RichText {
    egui::RichText::new(icon).font(egui::FontId::new(
        16.0,
        egui::FontFamily::Name("phosphor_icons".into()),
    ))
}

// Helper function to draw an icon on top of a widget with proper phosphor font
pub fn draw_icon_overlay(
    ui: &mut egui::Ui,
    icon: &str,
    position: egui::Pos2,
    size: f32,
    color: egui::Color32,
) {
    ui.painter().text(
        position,
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::new(size, egui::FontFamily::Name("phosphor_icons".into())),
        color,
    );
}

// Helper function to create a button with icon and text using LayoutJob
pub fn button_with_icon(
    ui: &mut egui::Ui,
    icon: &str,
    text: &str,
    enabled: bool,
) -> egui::Response {
    // Create a LayoutJob to mix fonts properly
    let mut layout_job = egui::text::LayoutJob::default();

    // Add icon with phosphor font
    layout_job.append(
        icon,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(16.0, egui::FontFamily::Name("phosphor_icons".into())),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );

    // Add space
    layout_job.append(
        " ",
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(14.0, egui::FontFamily::Proportional),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );

    // Add text with normal font
    layout_job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(14.0, egui::FontFamily::Proportional),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );

    ui.add_enabled(enabled, egui::Button::new(layout_job))
}
