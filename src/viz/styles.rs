#[allow(dead_code)]
use eframe::egui::Color32;

// -- Deep Backgrounds ------------------------------------------------
pub const BG_DEEP: Color32 = Color32::from_rgb(13, 14, 21);       // #0D0E15
pub const BG_PANEL: Color32 = Color32::from_rgb(20, 22, 34);      // #141622
pub const BG_PANEL_HI: Color32 = Color32::from_rgb(30, 33, 50);   // hover state
pub const PANEL_BORDER: Color32 = Color32::from_rgb(42, 45, 66);  // #2A2D42

// Legacy aliases
pub const BG_PRIMARY: Color32 = BG_DEEP;
pub const PANEL_BG: Color32 = BG_PANEL;

// -- Accent Primary: Warm Gold / Amber --------------------------------
pub const ACCENT_GOLD: Color32 = Color32::from_rgb(245, 176, 65); // #F5B041

// -- Accent Dynamic: Soft Blue / Neon Violet --------------------------
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(112, 161, 255); // #70A1FF
pub const ACCENT_VIOLET: Color32 = Color32::from_rgb(165, 94, 234); // #A55EEA

// -- Meter Spectrum Gradient Stops ------------------------------------
pub const METER_GREEN: Color32 = Color32::from_rgb(46, 213, 115);  // #2ED573
pub const METER_AMBER: Color32 = Color32::from_rgb(255, 165, 2);   // #FFA502
pub const METER_CRIMSON: Color32 = Color32::from_rgb(255, 71, 87); // #FF4757

// -- Frequency Band Colors --------------------------------------------
pub const FREQ_LOW: Color32 = Color32::from_rgb(255, 71, 87);     // crimson for bass
pub const FREQ_MID: Color32 = Color32::from_rgb(245, 176, 65);    // gold for mids
pub const FREQ_HIGH: Color32 = Color32::from_rgb(165, 94, 234);   // violet for highs

// -- Text -------------------------------------------------------------
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(225, 228, 240);
pub const TEXT_DIM: Color32 = Color32::from_rgb(85, 90, 115);
pub const TEXT_SECONDARY: Color32 = TEXT_DIM;
pub const TEXT_GLOW: Color32 = ACCENT_GOLD;

// -- Waveform ---------------------------------------------------------
pub const WAVE_PLAYED: Color32 = Color32::from_rgba_premultiplied(112, 161, 255, 90);
pub const WAVE_UNPLAYED: Color32 = Color32::from_rgba_premultiplied(55, 58, 80, 45);
pub const WAVE_COLOR: Color32 = ACCENT_BLUE;
pub const WAVE_FILL: Color32 = WAVE_PLAYED;
pub const PLAYHEAD_COLOR: Color32 = ACCENT_GOLD;

// -- Spectrogram Colormap Stops (Navy -> Violet -> Magenta -> Amber -> White)
pub const SPEC_COLD: Color32 = Color32::from_rgb(10, 12, 30);
pub const SPEC_COOL: Color32 = Color32::from_rgb(80, 40, 140);
pub const SPEC_WARM: Color32 = Color32::from_rgb(200, 50, 120);
pub const SPEC_HOT: Color32 = Color32::from_rgb(255, 165, 2);
pub const SPEC_PEAK: Color32 = Color32::from_rgb(255, 245, 230);

// -- Radial & Meters --------------------------------------------------
pub const RADIAL_BASE: Color32 = Color32::from_rgba_premultiplied(112, 161, 255, 50);
pub const RMS_BAR: Color32 = ACCENT_GOLD;
pub const PEAK_HOLD: Color32 = Color32::from_rgb(255, 255, 255);
pub const PHASE_POSITIVE: Color32 = Color32::from_rgb(46, 213, 115);
pub const PHASE_NEGATIVE: Color32 = Color32::from_rgb(255, 71, 87);
pub const PERSISTENCE_FADE: Color32 = Color32::from_rgba_premultiplied(20, 22, 34, 40);

// -- Glow Helpers -----------------------------------------------------
pub const GLOW_CYAN: Color32 = Color32::from_rgba_premultiplied(112, 161, 255, 25);
pub const GLOW_MAGENTA: Color32 = Color32::from_rgba_premultiplied(255, 71, 87, 25);
pub const GLOW_VIOLET: Color32 = Color32::from_rgba_premultiplied(165, 94, 234, 25);

// -- Grid & Atmosphere ------------------------------------------------
pub const GRID_DOT: Color32 = Color32::from_rgba_premultiplied(112, 161, 255, 6);
pub const BEAT_FLASH: Color32 = Color32::from_rgba_premultiplied(245, 176, 65, 5);
pub const VIGNETTE_EDGE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 45);

// -- Buttons ----------------------------------------------------------
pub const BTN_PRIMARY_FILL: Color32 = ACCENT_GOLD;
pub const BTN_PRIMARY_TEXT: Color32 = Color32::from_rgb(13, 14, 21);
pub const BTN_SECONDARY_FILL: Color32 = BG_PANEL_HI;
pub const BTN_SECONDARY_TEXT: Color32 = TEXT_PRIMARY;
pub const BTN_HOVER_OVERLAY: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 12);
pub const BTN_STOP_TEXT: Color32 = METER_CRIMSON;

// -- Live Badge -------------------------------------------------------
pub const LIVE_ON: Color32 = Color32::from_rgb(255, 71, 87);
pub const LIVE_OFF: Color32 = TEXT_DIM;

// -- Backward-compat neon aliases -------------------------------------
pub const NEON_CYAN: Color32 = ACCENT_BLUE;
pub const NEON_MAGENTA: Color32 = METER_CRIMSON;
pub const NEON_VIOLET: Color32 = ACCENT_VIOLET;
pub const NEON_LIME: Color32 = METER_GREEN;
pub const NEON_AMBER: Color32 = METER_AMBER;
pub const BTN_OUTLINE_BORDER: Color32 = PANEL_BORDER;
pub const BTN_STOP_BORDER: Color32 = PANEL_BORDER;
pub const BTN_HOVER_GLOW: Color32 = BTN_HOVER_OVERLAY;

// -- Helper: color with custom alpha ----------------------------------
pub fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * a as f32 / 255.0) as u8,
        (c.g() as f32 * a as f32 / 255.0) as u8,
        (c.b() as f32 * a as f32 / 255.0) as u8,
        a,
    )
}

// -- Helper: lerp two colors ------------------------------------------
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

// -- Helper: meter spectrum gradient (green -> amber -> crimson) ------
pub fn meter_gradient(frac: f32) -> Color32 {
    let f = frac.clamp(0.0, 1.0);
    if f < 0.55 {
        lerp_color(METER_GREEN, METER_AMBER, f / 0.55)
    } else {
        lerp_color(METER_AMBER, METER_CRIMSON, (f - 0.55) / 0.45)
    }
}

// -- Helper: spectrogram colormap (navy -> violet -> magenta -> amber -> white)
pub fn spectro_colormap(t: f32) -> Color32 {
    let f = t.clamp(0.0, 1.0);
    if f < 0.2 {
        lerp_color(SPEC_COLD, SPEC_COOL, f / 0.2)
    } else if f < 0.45 {
        lerp_color(SPEC_COOL, SPEC_WARM, (f - 0.2) / 0.25)
    } else if f < 0.75 {
        lerp_color(SPEC_WARM, SPEC_HOT, (f - 0.45) / 0.3)
    } else {
        lerp_color(SPEC_HOT, SPEC_PEAK, (f - 0.75) / 0.25)
    }
}