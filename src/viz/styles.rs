use eframe::egui::Color32;

// Light Theme Base
pub const BG_PRIMARY: Color32 = Color32::from_rgb(248, 250, 252);    // Soft white
pub const BG_SECONDARY: Color32 = Color32::from_rgb(241, 245, 249); // Light gray
pub const PANEL_BG: Color32 = Color32::from_rgb(255, 255, 255);     // Pure white

// Accent Colors (Technical Audio Station)
pub const WAVE_COLOR: Color32 = Color32::from_rgb(59, 130, 246);   // Electric Blue
pub const FREQ_LOW: Color32 = Color32::from_rgb(239, 68, 68);      // Red (Bass)
pub const FREQ_MID: Color32 = Color32::from_rgb(245, 158, 11);     // Amber (Mids)
pub const FREQ_HIGH: Color32 = Color32::from_rgb(16, 185, 129);    // Emerald (Treble)

// UI Elements
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(30, 41, 59);   // Dark slate
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(100, 116, 139); // Medium gray
pub const GRID_COLOR: Color32 = Color32::from_rgb(226, 232, 240);  // Very light gray
pub const ACCENT_GLOW: Color32 = Color32::from_rgba_premultiplied(59, 130, 246, 40); // Blue glow