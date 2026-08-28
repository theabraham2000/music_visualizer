use eframe::egui::Color32;

// Dark Professional Audio Theme
pub const BG_PRIMARY: Color32 = Color32::from_rgb(30, 30, 46);      // Deep Charcoal
pub const PANEL_BG: Color32 = Color32::from_rgb(38, 38, 58);        // Panel surface

pub const WAVE_COLOR: Color32 = Color32::from_rgb(99, 179, 237);    // Cyan Blue
pub const WAVE_FILL: Color32 = Color32::from_rgba_premultiplied(99, 179, 237, 60); 

pub const FREQ_LOW: Color32 = Color32::from_rgb(255, 107, 107);     // Soft Red
pub const FREQ_MID: Color32 = Color32::from_rgb(255, 234, 167);     // Warm Yellow
pub const FREQ_HIGH: Color32 = Color32::from_rgb(129, 236, 236);    // Mint Cyan

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(220, 220, 230); // Off-white
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 160); // Muted gray

// New styles for Full Track Overview
pub const OVERVIEW_WAVE: Color32 = Color32::from_rgba_premultiplied(99, 179, 237, 40);
pub const PLAYHEAD_COLOR: Color32 = Color32::from_rgb(255, 255, 255);