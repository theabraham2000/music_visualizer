use eframe::egui::{self, Pos2, Rect, Stroke, Color32, Rounding};
use eframe::epaint::Mesh;
use super::styles;
use std::sync::LazyLock;

// Logarithmic frequency band edges (Hz) for 96 bars spanning 20Hz-20kHz
static LOG_BAND_EDGES: LazyLock<[f32; 97]> = LazyLock::new(|| {
    let min_hz = 20.0_f32;
    let max_hz = 20000.0_f32;
    let log_min = min_hz.ln();
    let log_max = max_hz.ln();
    let mut bands = [0.0_f32; 97];
    for i in 0..97 {
        let t = i as f32 / 96.0;
        bands[i] = (log_min + t * (log_max - log_min)).exp();
    }
    bands
});

#[allow(clippy::too_many_arguments)]
pub fn draw_visualizer(
    ui: &mut egui::Ui,
    overview_envelope: &[(f32, f32)],
    spectrum: &[f32],
    playhead: usize,
    total_frames: usize,
    spectrogram_history: &[Vec<f32>],
    _persistence_buffer: &[Vec<Pos2>],
    left_channel: &[f32],
    right_channel: &[f32],
    rms_db: f32,
    peak_db: f32,
    radial_rotation: f32,
    bass_pulse: f32,
    sample_rate: u32,
) {
    let full_width = ui.available_width();
    let available_h = ui.available_height();

    // FIXED: Use explicit fixed heights with guaranteed minimums
    // Total budget: radial(300) + gap(8) + meters(56) + gap(8) + spectro(140) + gap(8) + wave(80) = 600
    let radial_h = 300.0_f32.min(available_h * 0.50).max(200.0);
    let meter_h = 56.0_f32;
    let spectro_h = 140.0_f32.min(available_h * 0.22).max(80.0);
    let wave_h = 80.0_f32.max(available_h - radial_h - meter_h - spectro_h - 40.0);

    // ── HERO: Radial Spectrum ──────────────────────────────────────
    let (_, radial_rect) = ui.allocate_space(egui::vec2(full_width, radial_h));
    draw_radial_hero(ui.painter_at(radial_rect), radial_rect, spectrum, radial_rotation, bass_pulse, sample_rate);

    ui.add_space(8.0);

    // ── Secondary row: RMS (left) + Phase (right) ─────────────────
    ui.horizontal(|ui| {
        let half_w = full_width / 2.0 - 4.0;
        let (_, rms_rect) = ui.allocate_space(egui::vec2(half_w, meter_h));
        draw_rms_meter(ui.painter_at(rms_rect), rms_rect, rms_db, peak_db);

        ui.add_space(8.0);

        let (_, phase_rect) = ui.allocate_space(egui::vec2(half_w, meter_h));
        draw_phase_meter(ui.painter_at(phase_rect), phase_rect, left_channel, right_channel);
    });

    ui.add_space(8.0);

    // ── Spectrogram ────────────────────────────────────────────────
    let (_, spec_rect) = ui.allocate_space(egui::vec2(full_width, spectro_h));
    draw_spectrogram(ui.painter_at(spec_rect), spec_rect, spectrogram_history);

    ui.add_space(8.0);

    // ── Waveform Timeline ──────────────────────────────────────────
    let (_, wave_rect) = ui.allocate_space(egui::vec2(full_width, wave_h));
    draw_waveform_timeline(ui.painter_at(wave_rect), wave_rect, overview_envelope, playhead, total_frames);
}

// =============================================================================
// HERO RADIAL SPECTRUM
// =============================================================================
fn draw_radial_hero(painter: egui::Painter, rect: Rect, spectrum: &[f32], rotation: f32, bass_pulse: f32, sample_rate: u32) {
    painter.rect_filled(rect, Rounding::same(16.0), styles::PANEL_BG);

    let center = rect.center();
    let base_radius = rect.width().min(rect.height()) * 0.40;
    let pulse = 1.0 + bass_pulse * 0.12;
    let radius = base_radius * pulse;
    let inner_r = radius * 0.45;
    let bar_count = 96;
    let angle_step = std::f32::consts::TAU / bar_count as f32;
    let nyquist = sample_rate as f32 / 2.0;

    for i in 0..bar_count {
        let low_hz = LOG_BAND_EDGES[i];
        let high_hz = LOG_BAND_EDGES[i + 1];
        let bin_low = ((low_hz / nyquist) * spectrum.len() as f32) as usize;
        let bin_high = ((high_hz / nyquist) * spectrum.len() as f32) as usize;
        let bin_low = bin_low.min(spectrum.len().saturating_sub(1));
        let bin_high = bin_high.min(spectrum.len()).max(bin_low + 1);

        let band_mag: f32 = spectrum[bin_low..bin_high].iter().sum::<f32>() / (bin_high - bin_low) as f32;
        let db = if band_mag > 1e-6 { 20.0 * band_mag.log10() } else { -60.0 };
        let normalized = ((db + 60.0) / 60.0).clamp(0.0, 1.0);

        let angle = i as f32 * angle_step + rotation;
        let outer_r = inner_r + normalized * (radius - inner_r);

        let start = Pos2::new(center.x + angle.cos() * inner_r, center.y + angle.sin() * inner_r);
        let end = Pos2::new(center.x + angle.cos() * outer_r, center.y + angle.sin() * outer_r);

        let color = if i < bar_count / 3 {
            styles::FREQ_LOW
        } else if i < 2 * bar_count / 3 {
            styles::FREQ_MID
        } else {
            styles::FREQ_HIGH
        };

        painter.line_segment([start, end], Stroke::new(3.0, color));
    }

    // Inner ring
    painter.circle_stroke(center, inner_r - 2.0, Stroke::new(1.0, styles::RADIAL_BASE));

    // Center label
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        "MinSu Lab",
        egui::FontId::proportional(12.0),
        styles::TEXT_PRIMARY,
    );

    // Frequency markers around the circle
    let markers: [(f32, &str); 4] = [(20.0, "20"), (100.0, "100"), (1000.0, "1k"), (10000.0, "10k")];
    for (hz, label) in &markers {
        let frac = ((*hz).ln() - 20.0_f32.ln()) / (20000.0_f32.ln() - 20.0_f32.ln());
        let angle = frac * std::f32::consts::TAU + rotation;
        let label_r = radius + 16.0;
        let pos = Pos2::new(center.x + angle.cos() * label_r, center.y + angle.sin() * label_r);
        painter.text(pos, egui::Align2::CENTER_CENTER, *label, egui::FontId::proportional(8.0), styles::TEXT_SECONDARY);
    }
}

// =============================================================================
// SPECTROGRAM
// =============================================================================
fn draw_spectrogram(painter: egui::Painter, rect: Rect, history: &[Vec<f32>]) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);
    if history.is_empty() { return; }

    let cols = history.len();
    let rows = 128;
    let cell_w = rect.width() / cols as f32;
    let cell_h = rect.height() / rows as f32;

    let mut mesh = Mesh::default();
    for (col_idx, column) in history.iter().enumerate() {
        let x = rect.left() + col_idx as f32 * cell_w;
        for row in 0..rows {
            let bin_idx = ((row as f32 / rows as f32).powf(1.5) * column.len() as f32) as usize;
            let mag = column.get(bin_idx).copied().unwrap_or(0.0);
            let db = if mag > 1e-6 { 20.0 * mag.log10() + 60.0 } else { 0.0 };
            let t = (db / 60.0).clamp(0.0, 1.0);
            let color = inferno_colormap(t);
            let y = rect.bottom() - row as f32 * cell_h;
            let cr = Rect::from_min_size(Pos2::new(x, y - cell_h), egui::vec2(cell_w + 0.5, cell_h + 0.5));
            mesh.colored_vertex(cr.left_top(), color);
            mesh.colored_vertex(cr.right_top(), color);
            mesh.colored_vertex(cr.left_bottom(), color);
            mesh.colored_vertex(cr.right_bottom(), color);
            let idx = mesh.vertices.len() as u32 - 4;
            mesh.add_triangle(idx, idx + 1, idx + 2);
            mesh.add_triangle(idx + 1, idx + 3, idx + 2);
        }
    }
    painter.add(mesh);

    painter.text(rect.left_top() + egui::vec2(10.0, 6.0), egui::Align2::LEFT_TOP, "SPECTROGRAM", egui::FontId::proportional(9.0), styles::TEXT_SECONDARY);
}

fn inferno_colormap(t: f32) -> Color32 {
    if t < 0.25 { lerp_color(styles::SPEC_COLD, styles::SPEC_COOL, t / 0.25) }
    else if t < 0.5 { lerp_color(styles::SPEC_COOL, styles::SPEC_WARM, (t - 0.25) / 0.25) }
    else if t < 0.75 { lerp_color(styles::SPEC_WARM, styles::SPEC_HOT, (t - 0.5) / 0.25) }
    else { lerp_color(styles::SPEC_HOT, styles::SPEC_PEAK, (t - 0.75) / 0.25) }
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgba_premultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        255,
    )
}

// =============================================================================
// PHASE METER (Proper stereo goniometer)
// =============================================================================
fn draw_phase_meter(painter: egui::Painter, rect: Rect, left: &[f32], right: &[f32]) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);

    let center = rect.center();
    let size = (rect.width().min(rect.height()) / 2.0) - 10.0;

    painter.line_segment([Pos2::new(center.x - size, center.y), Pos2::new(center.x + size, center.y)], Stroke::new(0.5, styles::TEXT_SECONDARY));
    painter.line_segment([Pos2::new(center.x, center.y - size), Pos2::new(center.x, center.y + size)], Stroke::new(0.5, styles::TEXT_SECONDARY));

    let len = left.len().min(right.len()).min(1024);
    let mut points: Vec<Pos2> = Vec::with_capacity(len);
    for i in 0..len {
        let x = center.x + left[i] * size;
        let y = center.y - right[i] * size;
        points.push(Pos2::new(x.clamp(rect.left(), rect.right()), y.clamp(rect.top(), rect.bottom())));
    }

    if points.len() > 1 {
        for pts in points.windows(2) {
            painter.line_segment([pts[0], pts[1]], Stroke::new(1.0, styles::PHASE_POSITIVE));
        }
    }

    painter.text(rect.left_top() + egui::vec2(10.0, 6.0), egui::Align2::LEFT_TOP, "STEREO PHASE", egui::FontId::proportional(9.0), styles::TEXT_SECONDARY);
}

// =============================================================================
// RMS LOUDNESS METER
// =============================================================================
fn draw_rms_meter(painter: egui::Painter, rect: Rect, rms_db: f32, peak_db: f32) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);

    let margin = 12.0;
    let bar_left = rect.left() + margin;
    let bar_right = rect.right() - margin;
    let bar_top = rect.top() + 22.0;
    let bar_bottom = rect.bottom() - 8.0;
    let bar_height = bar_bottom - bar_top;
    let min_db = -60.0_f32;
    let max_db = 0.0_f32;

    let rms_frac = ((rms_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
    let peak_frac = ((peak_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);

    let track_rect = Rect::from_min_max(Pos2::new(bar_left, bar_top), Pos2::new(bar_right, bar_bottom));
    painter.rect_filled(track_rect, Rounding::same(4.0), Color32::from_gray(20));

    let fill_w = rms_frac * track_rect.width();
    painter.rect_filled(Rect::from_min_size(Pos2::new(bar_left, bar_top), egui::vec2(fill_w, bar_height)), Rounding::same(4.0), styles::RMS_BAR);

    let peak_x = bar_left + peak_frac * track_rect.width();
    painter.line_segment([Pos2::new(peak_x, bar_top - 2.0), Pos2::new(peak_x, bar_bottom + 2.0)], Stroke::new(2.0, styles::PEAK_HOLD));

    painter.text(rect.left_top() + egui::vec2(10.0, 4.0), egui::Align2::LEFT_TOP, "RMS LOUDNESS", egui::FontId::proportional(9.0), styles::TEXT_SECONDARY);
    painter.text(rect.right_top() + egui::vec2(-10.0, 4.0), egui::Align2::RIGHT_TOP, &format!("{:.1} dB", rms_db), egui::FontId::monospace(10.0), styles::TEXT_PRIMARY);
}

// =============================================================================
// WAVEFORM TIMELINE (Min/Max envelope)
// =============================================================================
fn draw_waveform_timeline(painter: egui::Painter, rect: Rect, envelope: &[(f32, f32)], playhead: usize, total_frames: usize) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);
    if envelope.is_empty() || total_frames == 0 { return; }

    let mid_y = rect.center().y;
    let width = rect.width() - 20.0;
    let start_x = rect.left() + 10.0;
    let half_h = rect.height() / 2.0 - 8.0;

    let mut mesh = Mesh::default();
    let step = width / envelope.len() as f32;
    for (i, &(min_s, max_s)) in envelope.iter().enumerate() {
        let x = start_x + i as f32 * step;
        let y_min = mid_y - min_s * half_h;
        let y_max = mid_y - max_s * half_h;
        mesh.colored_vertex(Pos2::new(x, y_max), styles::WAVE_FILL);
        mesh.colored_vertex(Pos2::new(x, y_min), styles::WAVE_FILL);
        if i > 0 {
            let idx = mesh.vertices.len() as u32 - 2;
            mesh.add_triangle(idx - 2, idx - 1, idx);
            mesh.add_triangle(idx - 1, idx + 1, idx);
        }
    }
    painter.add(mesh);

    let progress = (playhead as f32 / total_frames as f32).clamp(0.0, 1.0);
    let head_x = start_x + progress * width;
    painter.line_segment([Pos2::new(head_x, rect.top()), Pos2::new(head_x, rect.bottom())], Stroke::new(2.0, styles::PLAYHEAD_COLOR));

    painter.text(rect.left_top() + egui::vec2(10.0, 6.0), egui::Align2::LEFT_TOP, "WAVEFORM", egui::FontId::proportional(9.0), styles::TEXT_SECONDARY);
}