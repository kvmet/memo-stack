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
pub const SETTINGS: &str = "\u{e959}"; // Gear icon for settings
pub const DELAY: &str = "\u{E492}"; // Clock icon for delayed memos
pub const ARCHIVE: &str = "\u{e928}"; // Archive icon for cold storage
pub const SPOTLIGHT: &str = "\u{e952}"; // Eye icon for spotlight feature
pub const EXPAND: &str = "\u{E13A}"; // Caret up for expand
pub const COLLAPSE: &str = "\u{E136}"; // Caret down for collapse
pub const CLOSE: &str = "\u{e9b1}"; // X icon for closing/canceling
pub const ALWAYS_ON_TOP: &str = "\u{E3E2}"; // X icon for closing/canceling

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

// Helper function to render an icon with text (NOTE: icon will render in default font)
pub fn icon_with_text(icon: &str, text: &str) -> String {
    format!("{} {}", icon, text)
}

// Helper function to create a button with properly rendered icon and text
pub fn icon_text_button(icon: &str, text: &str) -> impl Fn(&mut egui::Ui) -> egui::Response {
    let icon_owned = icon.to_string();
    let text_owned = text.to_string();
    move |ui: &mut egui::Ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0; // Small spacing between icon and text
            ui.add(egui::Label::new(icon_text(&icon_owned)));
            if !text_owned.is_empty() {
                ui.label(&text_owned);
            }
        })
        .response
    }
}

// Helper function to add icon with text to UI with proper font rendering
pub fn add_icon_text_to_ui(ui: &mut egui::Ui, icon: &str, text: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0; // Small spacing between icon and text
        ui.add(egui::Label::new(icon_text(icon)));
        if !text.is_empty() {
            ui.label(text);
        }
    });
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

// Helper function to create a button with icon overlay
pub fn button_with_icon(
    ui: &mut egui::Ui,
    icon: &str,
    text: &str,
    enabled: bool,
) -> egui::Response {
    let button_text = format!("    {}", text); // Add space for icon
    let response = ui.add_enabled(enabled, egui::Button::new(button_text));

    if enabled {
        let icon_pos = response.rect.left_center() + egui::vec2(8.0, 0.0);
        draw_icon_overlay(ui, icon, icon_pos, 16.0, ui.visuals().text_color());
    }

    response
}
