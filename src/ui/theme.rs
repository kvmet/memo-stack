use eframe::egui;

pub fn configure_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark(); // Start with dark theme

    // Customize colors
    visuals.window_fill = egui::Color32::from_rgb(30, 30, 35); // Dark background
    visuals.panel_fill = egui::Color32::from_rgb(25, 25, 30); // Slightly darker panels
    visuals.faint_bg_color = egui::Color32::from_rgb(40, 40, 45); // Subtle backgrounds

    // Button colors
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(60, 60, 65);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(80, 80, 85);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(100, 100, 105);

    // Text colors
    visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::WHITE; // Default text color
    visuals.widgets.inactive.fg_stroke.color = egui::Color32::WHITE;
    visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
    visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;

    // Accent color (for selections, highlights, etc.)
    visuals.selection.bg_fill = egui::Color32::from_rgb(70, 130, 130);
    visuals.selection.stroke.color = egui::Color32::from_rgb(225, 225, 225);

    // Border colors
    visuals.widgets.inactive.bg_stroke.color = egui::Color32::from_rgb(80, 80, 85);
    visuals.widgets.hovered.bg_stroke.color = egui::Color32::from_rgb(120, 120, 125);

    visuals
}

pub fn configure_fonts(
    fonts: &mut egui::FontDefinitions,
    atkinson_font: &'static [u8],
    phosphor_icons: &'static [u8],
) {
    fonts.font_data.insert(
        "atkinson_mono".to_owned(),
        egui::FontData::from_static(atkinson_font).into(),
    );
    fonts.font_data.insert(
        "phosphor_icons".to_owned(),
        egui::FontData::from_static(phosphor_icons).into(),
    );

    // Set as default for all font families
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "atkinson_mono".to_owned());

    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, "atkinson_mono".to_owned());

    // Create custom font family for icons
    fonts.families.insert(
        egui::FontFamily::Name("phosphor_icons".into()),
        vec!["phosphor_icons".to_owned()],
    );
}
