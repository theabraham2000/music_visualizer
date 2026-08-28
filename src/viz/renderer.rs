use eframe::egui::{self, Pos2, Rect, Stroke, Color32, Rounding, Sense};
use eframe::epaint::{Mesh, PathShape};
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

const RMS_SEGMENTS: usize = 48;

/// Result of waveform interaction: optional seek fraction (0..1)
pub struct WaveformInteraction {
    pub seek_fraction: Option<f32>,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_visualizer(
    ui: &mut egui::Ui,
    overview_envelope: &[(f32, f32)],
    spectrum: &[f32],
    playhead: usize,
    total_frames: usize,
    spectrogram_history: &[Vec<f32>],
    persistence_buffer: &[Vec<Pos2>],
    left_channel: &[f32],
    right_channel: &[f32],
    rms_db: f32,
    peak_db: f32,
    radial_rotation: f32,
    bass_pulse: f32,
    sample_rate: u32,
    anim_time: f32,
    beat_flash: f32,
) -> WaveformInteraction {
    let full_width = ui.available_width();
    let available_h = ui.available_height().max(480.0);

    // Proportional layout budget
    let radial_h = (available_h * 0.40).clamp(170.0, 300.0);
    let meter_h = 62.0_f32;
    let spectro_h = (available_h * 0.20).clamp(90.0, 150.0);
    let wave_h = (available_h - radial_h - meter_h - spectro_h - 12.0).max(64.0);

    // Background atmosphere
    let bg_rect = Rect::from_min_size(ui.min_rect().left_top(), egui::vec2(full_width, available_h));
    draw_background(ui.painter_at(bg_rect), bg_rect, beat_flash);

    // Radial hero
    let (_, radial_rect) = ui.allocate_space(egui::vec2(full_width, radial_h));
    draw_radial_hero(ui.painter_at(radial_rect), radial_rect, spectrum, radial_rotation, bass_pulse, sample_rate, anim_time);

    ui.add_space(3.0);

    // Meters row
    let mut _phase_corr: f32 = 0.0;
    ui.horizontal(|ui| {
        let half_w = full_width / 2.0 - 2.0;
        let (_, rms_rect) = ui.allocate_space(egui::vec2(half_w, meter_h));
        draw_rms_meter(ui.painter_at(rms_rect), rms_rect, rms_db, peak_db);

        ui.add_space(4.0);

        let (_, phase_rect) = ui.allocate_space(egui::vec2(half_w, meter_h));
        _phase_corr = draw_phase_meter(ui.painter_at(phase_rect), phase_rect, left_channel, right_channel, persistence_buffer);
    });

    ui.add_space(3.0);

    // Spectrogram
    let (_, spec_rect) = ui.allocate_space(egui::vec2(full_width, spectro_h));
    draw_spectrogram(ui.painter_at(spec_rect), spec_rect, spectrogram_history);

    ui.add_space(3.0);

    // Waveform with interaction
    let (wave_rect, wave_response) = ui.allocate_exact_size(egui::vec2(full_width, wave_h), Sense::click_and_drag());
    let seek_fraction = draw_waveform_timeline(ui.painter_at(wave_rect), wave_rect, overview_envelope, playhead, total_frames, &wave_response);

    WaveformInteraction { seek_fraction }
}

// =============================================================================
// BACKGROUND ATMOSPHERE
// =============================================================================
fn draw_background(painter: egui::Painter, rect: Rect, beat_flash: f32) {
    painter.rect_filled(rect, Rounding::ZERO, styles::BG_DEEP);

    // Subtle dot grid
    let spacing = 30.0_f32;
    let cols = (rect.width() / spacing) as usize;
    let rows = (rect.height() / spacing) as usize;
    for col in 0..=cols {
        for row in 0..=rows {
            let x = rect.left() + col as f32 * spacing;
            let y = rect.top() + row as f32 * spacing;
            painter.circle_filled(Pos2::new(x, y), 0.7, styles::GRID_DOT);
        }
    }

    // Beat flash overlay
    if beat_flash > 0.01 {
        let alpha = (beat_flash * 8.0) as u8;
        painter.rect_filled(rect, Rounding::ZERO, Color32::from_rgba_premultiplied(245, 176, 65, alpha));
    }
}

// =============================================================================
// RADIAL HERO — Smooth arcs with scale rings
// =============================================================================
fn draw_radial_hero(
    painter: egui::Painter,
    rect: Rect,
    spectrum: &[f32],
    rotation: f32,
    bass_pulse: f32,
    sample_rate: u32,
    anim_time: f32,
) {
    // Panel background
    painter.rect_filled(rect, Rounding::same(12.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, styles::PANEL_BORDER));

    let center = rect.center();
    let base_radius = rect.width().min(rect.height()) * 0.35;
    let pulse = 1.0 + bass_pulse * 0.15;
    let radius = base_radius * pulse;
    let inner_r = radius * 0.45;
    let bar_count = 96;
    let angle_step = std::f32::consts::TAU / bar_count as f32;
    let nyquist = sample_rate as f32 / 2.0;

    let has_energy = spectrum.iter().any(|&x| x > 0.0001);
    let breathe = if !has_energy { 1.0 + (anim_time * 1.0).sin() * 0.02 } else { 1.0 };

    // Scale rings at -40dB, -20dB, 0dB (normalized: 0.33, 0.67, 1.0)
    let db_rings: [(f32, &str); 3] = [(0.33, "-40"), (0.67, "-20"), (1.0, "0")];
    for (frac, label) in &db_rings {
        let ring_r = inner_r + frac * (radius - inner_r);
        // Dashed circle via short arc segments
        let dash_count = 60;
        let dash_angle = std::f32::consts::TAU / dash_count as f32;
        for i in 0..dash_count {
            if i % 3 != 0 { continue; } // skip 2 of every 3 for dashed effect
            let a1 = i as f32 * dash_angle;
            let a2 = (i + 1) as f32 * dash_angle;
            let p1 = Pos2::new(center.x + a1.cos() * ring_r, center.y + a1.sin() * ring_r);
            let p2 = Pos2::new(center.x + a2.cos() * ring_r, center.y + a2.sin() * ring_r);
            painter.line_segment([p1, p2], Stroke::new(0.5, styles::with_alpha(styles::PANEL_BORDER, 60)));
        }
        // Label at top of ring
        let label_pos = Pos2::new(center.x, center.y - ring_r - 2.0);
        painter.text(label_pos, egui::Align2::CENTER_BOTTOM, *label, egui::FontId::proportional(6.0), styles::TEXT_DIM);
    }

    // Draw bars as smooth rounded pill shapes using PathShape arcs
    // Glow pass first, then sharp pass
    for pass in 0..2 {
        let is_glow = pass == 0;
        let bar_half_width = if is_glow { 3.5 } else { 2.0 };
        let alpha_mult: f32 = if is_glow { 0.2 } else { 1.0 };

        for i in 0..bar_count {
            let low_hz = LOG_BAND_EDGES[i];
            let high_hz = LOG_BAND_EDGES[i + 1];
            let bin_low = ((low_hz / nyquist) * spectrum.len() as f32) as usize;
            let bin_high = ((high_hz / nyquist) * spectrum.len() as f32) as usize;
            let bin_low = bin_low.min(spectrum.len().saturating_sub(1));
            let bin_high = bin_high.min(spectrum.len()).max(bin_low + 1);

            let band_mag: f32 = spectrum[bin_low..bin_high].iter().sum::<f32>() / (bin_high - bin_low) as f32;
            let db = if band_mag > 1e-6 { 20.0 * band_mag.log10() } else { -60.0 };
            let normalized = ((db + 60.0) / 60.0).clamp(0.0, 1.0) * breathe;

            if normalized < 0.01 && !is_glow { continue; }

            let angle = i as f32 * angle_step + rotation - std::f32::consts::FRAC_PI_2;
            let outer_r = inner_r + normalized * (radius - inner_r);

            let base_color = if i < bar_count / 3 {
                styles::FREQ_LOW
            } else if i < 2 * bar_count / 3 {
                styles::FREQ_MID
            } else {
                styles::FREQ_HIGH
            };

            let color = if is_glow {
                styles::with_alpha(base_color, (alpha_mult * 255.0) as u8)
            } else {
                styles::lerp_color(base_color, Color32::WHITE, normalized * 0.35)
            };

            // Draw as a thick line segment with rounded appearance
            // For anti-aliased look, we draw the bar as a thin filled quad (pill shape)
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            // Perpendicular direction for bar width
            let perp_x = -sin_a;
            let perp_y = cos_a;

            let sx = center.x + cos_a * inner_r;
            let sy = center.y + sin_a * inner_r;
            let ex = center.x + cos_a * outer_r;
            let ey = center.y + sin_a * outer_r;

            // Four corners of the bar quad
            let hw = bar_half_width;
            let p1 = Pos2::new(sx + perp_x * hw, sy + perp_y * hw);
            let _p2 = Pos2::new(ex + perp_x * hw, ey + perp_y * hw);
            let _p3 = Pos2::new(ex - perp_x * hw, ey - perp_y * hw);
            let p4 = Pos2::new(sx - perp_x * hw, sy - perp_y * hw);

            // Rounded tip: add arc points at the outer end
            let tip_segments = 4;
            let mut points: Vec<Pos2> = vec![p1];
            for t in 0..=tip_segments {
                let frac = t as f32 / tip_segments as f32;
                let tip_angle = angle - std::f32::consts::PI + frac * std::f32::consts::PI;
                let tx = ex + tip_angle.cos() * hw;
                let ty = ey + tip_angle.sin() * hw;
                points.push(Pos2::new(tx, ty));
            }
            points.push(p4);

            let shape = PathShape::convex_polygon(points, color, Stroke::NONE);
            painter.add(shape);
        }
    }

    // Inner ring — smooth circle with subtle pulse
    let inner_ring_r = inner_r - 2.0;
    let ring_pulse = 1.0 + bass_pulse * 0.05;
    painter.circle_stroke(center, inner_ring_r * ring_pulse, Stroke::new(1.0, styles::RADIAL_BASE));

    // Outer boundary ring
    painter.circle_stroke(center, radius + 4.0, Stroke::new(0.5, styles::with_alpha(styles::PANEL_BORDER, 40)));

    // Center readout with surrounding ring indicator
    let center_text = if has_energy {
        let avg: f32 = spectrum.iter().sum::<f32>() / spectrum.len().max(1) as f32;
        let db_val = if avg > 1e-6 { 20.0 * avg.log10() } else { -60.0 };
        format!("{:.0} dB", db_val)
    } else {
        "MinSu".to_string()
    };
    let text_color = if has_energy { styles::ACCENT_GOLD } else { styles::TEXT_DIM };

    // Subtle ring around center text that pulses with bass
    let text_ring_r = inner_r * 0.38;
    let text_ring_alpha = (40.0 + bass_pulse * 60.0) as u8;
    painter.circle_stroke(center, text_ring_r, Stroke::new(1.0, styles::with_alpha(styles::ACCENT_GOLD, text_ring_alpha)));

    painter.text(center, egui::Align2::CENTER_CENTER, center_text, egui::FontId::monospace(12.0), text_color);

    // Frequency band labels along outer circumference
    let band_labels: [(f32, f32, &str, Color32); 3] = [
        (0.0, 1.0 / 3.0, "BASS", styles::FREQ_LOW),
        (1.0 / 3.0, 2.0 / 3.0, "MID", styles::FREQ_MID),
        (2.0 / 3.0, 1.0, "HIGH", styles::FREQ_HIGH),
    ];
    for (start_frac, end_frac, label, color) in &band_labels {
        let mid_frac = (start_frac + end_frac) / 2.0;
        let angle = mid_frac * std::f32::consts::TAU + rotation - std::f32::consts::FRAC_PI_2;
        let label_r = radius + 16.0;
        let pos = Pos2::new(center.x + angle.cos() * label_r, center.y + angle.sin() * label_r);
        painter.text(pos, egui::Align2::CENTER_CENTER, *label, egui::FontId::proportional(7.0), *color);
    }

    // Anchor frequency labels
    let markers: [(f32, &str); 4] = [(20.0, "20"), (200.0, "200"), (2000.0, "2k"), (20000.0, "20k")];
    for (hz, label) in &markers {
        let frac = ((*hz).ln() - 20.0_f32.ln()) / (20000.0_f32.ln() - 20.0_f32.ln());
        let angle = frac * std::f32::consts::TAU + rotation - std::f32::consts::FRAC_PI_2;
        let label_r = radius + 28.0;
        let pos = Pos2::new(center.x + angle.cos() * label_r, center.y + angle.sin() * label_r);
        painter.text(pos, egui::Align2::CENTER_CENTER, *label, egui::FontId::proportional(6.0), styles::TEXT_DIM);
    }

    // Spark particles on strong bass
    if bass_pulse > 0.7 {
        let spark_count = 8;
        for i in 0..spark_count {
            let angle = (i as f32 / spark_count as f32) * std::f32::consts::TAU + anim_time * 1.2;
            let dist = radius + 6.0 + bass_pulse * 15.0;
            let pos = Pos2::new(center.x + angle.cos() * dist, center.y + angle.sin() * dist);
            let alpha = ((bass_pulse - 0.7) / 0.3 * 100.0) as u8;
            painter.circle_filled(pos, 1.2, styles::with_alpha(styles::ACCENT_GOLD, alpha));
        }
    }
}

// =============================================================================
// SPECTROGRAM — High dynamic range colormap with grid lines
// =============================================================================
fn draw_spectrogram(painter: egui::Painter, rect: Rect, history: &[Vec<f32>]) {
    painter.rect_filled(rect, Rounding::same(10.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, styles::PANEL_BORDER));

    if history.is_empty() {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "SPECTROGRAM", egui::FontId::proportional(9.0), styles::TEXT_DIM);
        return;
    }

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
            let color = styles::spectro_colormap(t);
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

    // Horizontal grid lines at octave divisions
    let octaves: [(f32, &str); 4] = [
        (0.0, "20 kHz"),
        (0.33, "2 kHz"),
        (0.66, "200 Hz"),
        (0.92, "20 Hz"),
    ];
    for (frac, label) in &octaves {
        let y = rect.top() + frac * rect.height();
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5, styles::with_alpha(styles::PANEL_BORDER, 50)),
        );
        painter.text(
            Pos2::new(rect.left() + 3.0, y - 1.0),
            egui::Align2::LEFT_BOTTOM,
            *label,
            egui::FontId::proportional(6.0),
            styles::TEXT_DIM,
        );
    }

    // Panel label
    painter.text(
        rect.right_top() + egui::vec2(-8.0, 4.0),
        egui::Align2::RIGHT_TOP,
        "SPECTROGRAM",
        egui::FontId::proportional(8.0),
        styles::TEXT_DIM,
    );
}

// =============================================================================
// STEREO PHASE METER — Autoscaled goniometer with correlation readout
// =============================================================================
fn draw_phase_meter(
    painter: egui::Painter,
    rect: Rect,
    left: &[f32],
    right: &[f32],
    _persistence_buffer: &[Vec<Pos2>],
) -> f32 {
    painter.rect_filled(rect, Rounding::same(10.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, styles::PANEL_BORDER));

    let center = rect.center();
    // Use 78% of available space for the scope
    let size = (rect.width().min(rect.height()) / 2.0) * 0.78;

    // Circular boundary
    painter.circle_stroke(center, size, Stroke::new(0.5, styles::with_alpha(styles::PANEL_BORDER, 80)));

    // Crosshair axes
    painter.line_segment(
        [Pos2::new(center.x - size, center.y), Pos2::new(center.x + size, center.y)],
        Stroke::new(0.5, styles::with_alpha(styles::TEXT_DIM, 50)),
    );
    painter.line_segment(
        [Pos2::new(center.x, center.y - size), Pos2::new(center.x, center.y + size)],
        Stroke::new(0.5, styles::with_alpha(styles::TEXT_DIM, 50)),
    );

    // 45-degree diagonal guides for M/S alignment
    let d = size * 0.707;
    painter.line_segment(
        [Pos2::new(center.x - d, center.y - d), Pos2::new(center.x + d, center.y + d)],
        Stroke::new(0.3, styles::with_alpha(styles::PHASE_POSITIVE, 20)),
    );
    painter.line_segment(
        [Pos2::new(center.x - d, center.y + d), Pos2::new(center.x + d, center.y - d)],
        Stroke::new(0.3, styles::with_alpha(styles::PHASE_NEGATIVE, 20)),
    );

    let len = left.len().min(right.len());
    if len < 2 {
        painter.text(rect.left_top() + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP, "STEREO PHASE", egui::FontId::proportional(8.0), styles::TEXT_DIM);
        return 0.0;
    }

    // Autoscale: find peak amplitude to normalize display
    let mut peak_amp: f32 = 0.001;
    let max_samples = 512.min(len);
    let step = (len / max_samples).max(1);
    for i in (0..len).step_by(step) {
        peak_amp = peak_amp.max(left[i].abs()).max(right[i].abs());
    }
    let scale_factor = (0.8 / peak_amp).min(1.0); // autoscale to fill 80% of scope

    // Subsample for clean trace
    let mut points: Vec<Pos2> = Vec::with_capacity(max_samples);
    let mut correlation_sum = 0.0_f32;
    let mut energy_sum = 0.0_f32;

    for i in (0..len).step_by(step) {
        let lx = left[i] * scale_factor;
        let ry = right[i] * scale_factor;
        let x = center.x + lx * size;
        let y = center.y - ry * size;
        points.push(Pos2::new(
            x.clamp(rect.left() + 2.0, rect.right() - 2.0),
            y.clamp(rect.top() + 2.0, rect.bottom() - 2.0),
        ));
        correlation_sum += left[i] * right[i];
        energy_sum += left[i] * left[i] + right[i] * right[i];
    }

    let correlation = if energy_sum > 1e-6 {
        (correlation_sum / (energy_sum * 0.5)).clamp(-1.0, 1.0)
    } else {
        0.0
    };

    // Color trace by correlation value
    let trace_color = if correlation > 0.3 {
        styles::lerp_color(styles::ACCENT_BLUE, styles::PHASE_POSITIVE, (correlation - 0.3) / 0.7)
    } else if correlation < -0.3 {
        styles::lerp_color(styles::ACCENT_BLUE, styles::PHASE_NEGATIVE, (-correlation - 0.3) / 0.7)
    } else {
        styles::ACCENT_BLUE
    };

    // Draw trace
    if points.len() > 1 {
        for pts in points.windows(2) {
            painter.line_segment([pts[0], pts[1]], Stroke::new(1.0, styles::with_alpha(trace_color, 160)));
        }
    }

    // Correlation readout (top-right)
    let corr_color = if correlation > 0.3 { styles::PHASE_POSITIVE } else if correlation < -0.3 { styles::PHASE_NEGATIVE } else { styles::ACCENT_BLUE };
    painter.text(
        rect.right_top() + egui::vec2(-8.0, 16.0),
        egui::Align2::RIGHT_TOP,
        format!("{:+.2}", correlation),
        egui::FontId::monospace(9.0),
        corr_color,
    );

    // Panel label
    painter.text(rect.left_top() + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP, "STEREO PHASE", egui::FontId::proportional(8.0), styles::TEXT_DIM);

    correlation
}

// =============================================================================
// RMS LOUDNESS METER — Continuous gradient bar with dB markings
// =============================================================================
fn draw_rms_meter(painter: egui::Painter, rect: Rect, rms_db: f32, peak_db: f32) {
    painter.rect_filled(rect, Rounding::same(10.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, styles::PANEL_BORDER));

    let margin = 10.0_f32;
    let bar_left = rect.left() + margin;
    let bar_right = rect.right() - margin - 55.0;
    let bar_top = rect.top() + 20.0;
    let bar_bottom = rect.bottom() - 14.0;
    let bar_height = bar_bottom - bar_top;
    let bar_width = bar_right - bar_left;
    let min_db = -60.0_f32;
    let max_db = 0.0_f32;

    let rms_frac = ((rms_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
    let peak_frac = ((peak_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);

    // Track background
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(bar_left, bar_top), Pos2::new(bar_right, bar_bottom)),
        Rounding::same(3.0),
        Color32::from_gray(8),
    );

    // Continuous gradient fill using mesh
    let fill_w = rms_frac * bar_width;
    if fill_w > 1.0 {
        let grad_steps = 32;
        let mut mesh = Mesh::default();
        for i in 0..=grad_steps {
            let t = i as f32 / grad_steps as f32;
            let x = bar_left + t * fill_w;
            let color = styles::meter_gradient(t * rms_frac);
            mesh.colored_vertex(Pos2::new(x, bar_top), color);
            mesh.colored_vertex(Pos2::new(x, bar_bottom), color);
            if i > 0 {
                let idx = mesh.vertices.len() as u32 - 2;
                mesh.add_triangle(idx - 2, idx - 1, idx);
                mesh.add_triangle(idx - 1, idx + 1, idx);
            }
        }
        painter.add(mesh);

        // Thin divider overlays for segment feel
        let seg_count = RMS_SEGMENTS;
        let active_segs = (rms_frac * seg_count as f32) as usize;
        for i in 0..active_segs {
            let x = bar_left + (i + 1) as f32 / seg_count as f32 * fill_w;
            if x < bar_left + fill_w {
                painter.line_segment(
                    [Pos2::new(x, bar_top), Pos2::new(x, bar_bottom)],
                    Stroke::new(0.5, styles::with_alpha(styles::BG_DEEP, 120)),
                );
            }
        }
    }

    // Peak hold indicator with smooth visual
    let peak_x = bar_left + peak_frac * bar_width;
    painter.line_segment(
        [Pos2::new(peak_x, bar_top - 1.0), Pos2::new(peak_x, bar_bottom + 1.0)],
        Stroke::new(1.5, styles::PEAK_HOLD),
    );

    // dB scale markings below the bar
    let db_marks: [f32; 7] = [-60.0, -48.0, -36.0, -24.0, -12.0, -6.0, 0.0];
    for &db_val in &db_marks {
        let frac = ((db_val - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
        let x = bar_left + frac * bar_width;
        painter.line_segment(
            [Pos2::new(x, bar_bottom + 1.0), Pos2::new(x, bar_bottom + 3.5)],
            Stroke::new(0.5, styles::TEXT_DIM),
        );
        let label = if db_val == 0.0 { "0".to_string() } else { format!("{}", db_val as i32) };
        painter.text(
            Pos2::new(x, bar_bottom + 5.0),
            egui::Align2::CENTER_TOP,
            label,
            egui::FontId::proportional(5.5),
            styles::TEXT_DIM,
        );
    }

    // Large dB readout
    painter.text(
        Pos2::new(rect.right() - margin, bar_top + bar_height / 2.0),
        egui::Align2::RIGHT_CENTER,
        format!("{:.1}", rms_db),
        egui::FontId::monospace(13.0),
        styles::ACCENT_GOLD,
    );
    painter.text(
        Pos2::new(rect.right() - margin, bar_top + bar_height / 2.0 + 10.0),
        egui::Align2::RIGHT_CENTER,
        "dB",
        egui::FontId::proportional(7.0),
        styles::TEXT_DIM,
    );

    // Panel label
    painter.text(rect.left_top() + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP, "RMS LOUDNESS", egui::FontId::proportional(8.0), styles::TEXT_DIM);
}

// =============================================================================
// WAVEFORM TIMELINE — Interactive dual-tone envelope with glowing playhead
// =============================================================================
fn draw_waveform_timeline(
    painter: egui::Painter,
    rect: Rect,
    envelope: &[(f32, f32)],
    playhead: usize,
    total_frames: usize,
    response: &egui::Response,
) -> Option<f32> {
    painter.rect_filled(rect, Rounding::same(10.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, styles::PANEL_BORDER));

    if envelope.is_empty() || total_frames == 0 {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "WAVEFORM", egui::FontId::proportional(9.0), styles::TEXT_DIM);
        return None;
    }

    let mid_y = rect.center().y;
    let margin_x = 8.0_f32;
    let width = rect.width() - margin_x * 2.0;
    let start_x = rect.left() + margin_x;
    let half_h = rect.height() / 2.0 - 8.0;
    let progress = (playhead as f32 / total_frames as f32).clamp(0.0, 1.0);
    let head_x = start_x + progress * width;
    let step = width / envelope.len() as f32;

    // Played region fill (brighter)
    let mut played_mesh = Mesh::default();
    let mut unplayed_mesh = Mesh::default();

    for (i, &(min_s, max_s)) in envelope.iter().enumerate() {
        let x = start_x + i as f32 * step;
        let y_min = mid_y - min_s * half_h;
        let y_max = mid_y - max_s * half_h;
        let is_played = x <= head_x;
        let fill_color = if is_played { styles::WAVE_PLAYED } else { styles::WAVE_UNPLAYED };
        let target_mesh = if is_played { &mut played_mesh } else { &mut unplayed_mesh };

        target_mesh.colored_vertex(Pos2::new(x, y_max), fill_color);
        target_mesh.colored_vertex(Pos2::new(x, y_min), fill_color);

        if i > 0 {
            let prev_x = start_x + (i - 1) as f32 * step;
            if (prev_x <= head_x) == is_played {
                let idx = target_mesh.vertices.len() as u32 - 2;
                target_mesh.add_triangle(idx - 2, idx - 1, idx);
                target_mesh.add_triangle(idx - 1, idx + 1, idx);
            }
        }
    }
    painter.add(unplayed_mesh);
    painter.add(played_mesh);

    // Envelope outline stroke
    for (i, &(min_s, max_s)) in envelope.iter().enumerate() {
        if i == 0 { continue; }
        let x = start_x + i as f32 * step;
        let prev_x = start_x + (i - 1) as f32 * step;
        let played = x <= head_x;
        let color = if played {
            styles::with_alpha(styles::ACCENT_BLUE, 110)
        } else {
            styles::with_alpha(styles::TEXT_DIM, 35)
        };
        let (prev_min, prev_max) = envelope[i - 1];
        painter.line_segment(
            [Pos2::new(prev_x, mid_y - prev_max * half_h), Pos2::new(x, mid_y - max_s * half_h)],
            Stroke::new(0.7, color),
        );
        painter.line_segment(
            [Pos2::new(prev_x, mid_y - prev_min * half_h), Pos2::new(x, mid_y - min_s * half_h)],
            Stroke::new(0.7, color),
        );
    }

    // Full-height glowing playhead line
    // Glow pass
    painter.line_segment(
        [Pos2::new(head_x, rect.top() + 2.0), Pos2::new(head_x, rect.bottom() - 2.0)],
        Stroke::new(5.0, styles::with_alpha(styles::ACCENT_GOLD, 20)),
    );
    // Sharp line
    painter.line_segment(
        [Pos2::new(head_x, rect.top() + 2.0), Pos2::new(head_x, rect.bottom() - 2.0)],
        Stroke::new(1.5, styles::PLAYHEAD_COLOR),
    );

    // Rounded top marker (small circle at top of playhead)
    painter.circle_filled(Pos2::new(head_x, rect.top() + 4.0), 3.0, styles::ACCENT_GOLD);

    // Hover timestamp tooltip
    if response.hovered() {
        if let Some(pointer_pos) = response.hover_pos() {
            let hover_frac = ((pointer_pos.x - start_x) / width).clamp(0.0, 1.0);
            let hover_secs = hover_frac * (total_frames as f32 / 44100.0); // approximate
            let m = (hover_secs / 60.0) as u32;
            let s = (hover_secs % 60.0) as u32;
            let tooltip_text = format!("{:02}:{:02}", m, s);
            let tooltip_pos = Pos2::new(pointer_pos.x, rect.top() - 2.0);
            painter.text(tooltip_pos, egui::Align2::CENTER_BOTTOM, tooltip_text, egui::FontId::monospace(9.0), styles::ACCENT_GOLD);
        }
    }

    // Panel label
    painter.text(rect.left_top() + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP, "WAVEFORM", egui::FontId::proportional(8.0), styles::TEXT_DIM);

    // Handle click/drag seeking
    if response.clicked() || response.dragged() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let seek_frac = ((pointer_pos.x - start_x) / width).clamp(0.0, 1.0);
            return Some(seek_frac);
        }
    }

    None
}