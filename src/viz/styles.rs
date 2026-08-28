use eframe::egui::Color32;

// Dark Professional Audio Theme
pub const BG_PRIMARY: Color32 = Color32::from_rgb(30, 30, 46);
pub const PANEL_BG: Color32 = Color32::from_rgb(38, 38, 58);

pub const WAVE_COLOR: Color32 = Color32::from_rgb(99, 179, 237);
pub const WAVE_FILL: Color32 = Color32::from_rgba_premultiplied(99, 179, 237, 60);

pub const FREQ_LOW: Color32 = Color32::from_rgb(255, 107, 107);
pub const FREQ_MID: Color32 = Color32::from_rgb(255, 234, 167);
pub const FREQ_HIGH: Color32 = Color32::from_rgb(129, 236, 236);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(220, 220, 230);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 160);

// Track Overview
pub const OVERVIEW_WAVE: Color32 = Color32::from_rgba_premultiplied(99, 179, 237, 40);
pub const PLAYHEAD_COLOR: Color32 = Color32::from_rgb(255, 255, 255);

// Spectrogram "Inferno" Colormap Stops
pub const SPEC_COLD: Color32 = Color32::from_rgb(0, 0, 0);       // Silence
pub const SPEC_COOL: Color32 = Color32::from_rgb(80, 20, 120);   // Low energy
pub const SPEC_WARM: Color32 = Color32::from_rgb(200, 50, 30);   // Mid energy
pub const SPEC_HOT: Color32 = Color32::from_rgb(255, 200, 50);   // High energy
pub const SPEC_PEAK: Color32 = Color32::from_rgb(255, 255, 240); // Peak

// Radial & Meters
pub const RADIAL_BASE: Color32 = Color32::from_rgba_premultiplied(99, 179, 237, 100);
pub const RMS_BAR: Color32 = Color32::from_rgb(99, 179, 237);
pub const PEAK_HOLD: Color32 = Color32::from_rgb(255, 107, 107);
pub const PHASE_POSITIVE: Color32 = Color32::from_rgb(129, 236, 236);
pub const PERSISTENCE_FADE: Color32 = Color32::from_rgba_premultiplied(38, 38, 58, 40);