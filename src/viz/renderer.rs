use eframe::egui::{self, Pos2, Rect, Stroke, Color32, Rounding};
use eframe::epaint::{PathShape, Mesh};
use super::styles;

pub fn draw_visualizer(
    ui: &mut egui::Ui, 
    samples: &[f32], 
    spectrum: &[f32], 
    playhead: usize, 
    total_samples: usize,
    spectrogram_history: &[Vec<f32>],
    persistence_buffer: &[Vec<Pos2>],
    rms_db: f32,
    peak_db: f32,
    radial_rotation: f32,
) {
    ui.vertical_centered(|ui| {
        let max_height = ui.available_height();
        let full_width = ui.available_width();
        
        // Top: Track Overview
        draw_track_overview(ui, samples, playhead, total_samples, 50.0);
        ui.add_space(8.0);
        
        // Middle Row: Spectrogram (left) + Radial (center) + Phase/RMS (right)
        let mid_row_height = (max_height * 0.35).min(220.0);
        ui.horizontal(|ui| {
            let col_w = full_width / 3.0 - 5.0;
            
            // Left: Spectrogram
            let (_, spec_rect) = ui.allocate_space(egui::vec2(col_w, mid_row_height));
            draw_spectrogram(ui.painter_at(spec_rect), spec_rect, spectrogram_history);
            
            ui.add_space(8.0);
            
            // Center: Radial Spectrum
            let (_, rad_rect) = ui.allocate_space(egui::vec2(col_w, mid_row_height));
            draw_radial_spectrum(ui.painter_at(rad_rect), rad_rect, spectrum, radial_rotation);
            
            ui.add_space(8.0);
            
            // Right Column: Phase Meter (top half) + RMS Meter (bottom half)
            ui.vertical(|ui| {
                let half_h = mid_row_height / 2.0 - 4.0;
                
                let (_, phase_rect) = ui.allocate_space(egui::vec2(col_w, half_h));
                draw_phase_meter(ui.painter_at(phase_rect), phase_rect, samples, playhead);
                
                ui.add_space(8.0);
                
                let (_, rms_rect) = ui.allocate_space(egui::vec2(col_w, half_h));
                draw_rms_meter(ui.painter_at(rms_rect), rms_rect, rms_db, peak_db);
            });
        });
        
        ui.add_space(8.0);
        
        // Bottom Row: Persistence Oscilloscope (left) + Standard Waveform/Spectrum (right)
        let bot_row_height = (max_height * 0.35).min(200.0);
        ui.horizontal(|ui| {
            let half_w = full_width / 2.0 - 4.0;
            
            // Left: Persistence Oscilloscope
            let (_, persist_rect) = ui.allocate_space(egui::vec2(half_w, bot_row_height));
            draw_persistence_oscilloscope(ui.painter_at(persist_rect), persist_rect, persistence_buffer);
            
            ui.add_space(8.0);
            
            // Right: Standard panels stacked
            ui.vertical(|ui| {
                let half_h = bot_row_height / 2.0 - 4.0;
                
                let (_, wave_rect) = ui.allocate_space(egui::vec2(half_w, half_h));
                draw_waveform_mini(ui.painter_at(wave_rect), wave_rect, samples, playhead);
                
                ui.add_space(8.0);
                
                let (_, spec_bar_rect) = ui.allocate_space(egui::vec2(half_w, half_h));
                draw_spectrum_bars_mini(ui.painter_at(spec_bar_rect), spec_bar_rect, spectrum);
            });
        });
    });
}


// =============================================================================
// 1. SPECTROGRAM (Heatmap)
// =============================================================================
fn draw_spectrogram(painter: egui::Painter, rect: Rect, history: &[Vec<f32>]) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);
    
    if history.is_empty() { return; }
    
    let cols = history.len();
    let rows = 128; // Frequency bins to display
    let cell_w = rect.width() / cols as f32;
    let cell_h = rect.height() / rows as f32;
    
    let mut mesh = Mesh::default();
    
    for (col_idx, column) in history.iter().enumerate() {
        let x = rect.left() + col_idx as f32 * cell_w;
        
        for row in 0..rows {
            // Map row to FFT bin index (logarithmic-ish mapping for better visual)
            let bin_idx = ((row as f32 / rows as f32).powf(1.5) * (column.len() as f32)) as usize;
            let mag = column.get(bin_idx).copied().unwrap_or(0.0);
            
            // Convert magnitude to dB-like scale for colormap
            let db = if mag > 0.0 { 20.0 * mag.log10() + 60.0 } else { 0.0 };
            let t = (db / 60.0).clamp(0.0, 1.0);
            
            let color = inferno_colormap(t);
            
            let y = rect.bottom() - (row as f32 * cell_h);
            let cell_rect = Rect::from_min_size(Pos2::new(x, y - cell_h), egui::vec2(cell_w + 0.5, cell_h + 0.5));
            
            mesh.colored_vertex(cell_rect.left_top(), color);
            mesh.colored_vertex(cell_rect.right_top(), color);
            mesh.colored_vertex(cell_rect.left_bottom(), color);
            mesh.colored_vertex(cell_rect.right_bottom(), color);
            
            let idx = mesh.vertices.len() as u32 - 4;
            mesh.add_triangle(idx, idx+1, idx+2);
            mesh.add_triangle(idx+1, idx+3, idx+2);
        }
    }
    
    painter.add(mesh);
    
    painter.text(
        rect.left_top() + egui::vec2(10.0, 8.0),
        egui::Align2::LEFT_TOP,
        "SPECTROGRAM",
        egui::FontId::proportional(9.0),
        styles::TEXT_SECONDARY,
    );
}

fn inferno_colormap(t: f32) -> Color32 {
    // Simplified inferno: black -> purple -> red -> yellow -> white
    if t < 0.25 {
        let s = t / 0.25;
        lerp_color(styles::SPEC_COLD, styles::SPEC_COOL, s)
    } else if t < 0.5 {
        let s = (t - 0.25) / 0.25;
        lerp_color(styles::SPEC_COOL, styles::SPEC_WARM, s)
    } else if t < 0.75 {
        let s = (t - 0.5) / 0.25;
        lerp_color(styles::SPEC_WARM, styles::SPEC_HOT, s)
    } else {
        let s = (t - 0.75) / 0.25;
        lerp_color(styles::SPEC_HOT, styles::SPEC_PEAK, s)
    }
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
// 2. CIRCULAR / RADIAL SPECTRUM
// =============================================================================
fn draw_radial_spectrum(painter: egui::Painter, rect: Rect, spectrum: &[f32], rotation: f32) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);
    
    let center = rect.center();
    let radius = (rect.width().min(rect.height()) / 2.0) - 20.0;
    let bar_count = 64;
    let angle_step = std::f32::consts::TAU / bar_count as f32;
    
    for i in 0..bar_count {
        let idx = (i as f32 / bar_count as f32 * spectrum.len() as f32) as usize;
        let mag = spectrum.get(idx).copied().unwrap_or(0.0);
        let adjusted = (mag * 80.0).powf(0.75).min(1.0);
        
        let angle = i as f32 * angle_step + rotation;
        let inner_r = radius * 0.3;
        let outer_r = inner_r + adjusted * radius * 0.7;
        
        let start = Pos2::new(
            center.x + angle.cos() * inner_r,
            center.y + angle.sin() * inner_r,
        );
        let end = Pos2::new(
            center.x + angle.cos() * outer_r,
            center.y + angle.sin() * outer_r,
        );
        
        let color = if i < bar_count / 3 { styles::FREQ_LOW }
                    else if i < 2 * bar_count / 3 { styles::FREQ_MID }
                    else { styles::FREQ_HIGH };
        
        painter.line_segment([start, end], Stroke::new(2.5, color));
    }
    
    // Inner glow circle
    painter.circle_stroke(center, radius * 0.28, Stroke::new(1.0, styles::RADIAL_BASE));
    
    painter.text(
        rect.left_top() + egui::vec2(10.0, 8.0),
        egui::Align2::LEFT_TOP,
        "RADIAL SPECTRUM",
        egui::FontId::proportional(9.0),
        styles::TEXT_SECONDARY,
    );
}

// =============================================================================
// 3. STEREO CORRELATION / PHASE METER (Goniometer)
// =============================================================================
fn draw_phase_meter(painter: egui::Painter, rect: Rect, samples: &[f32], playhead: usize) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);
    
    let center = rect.center();
    let size = (rect.width().min(rect.height()) / 2.0) - 15.0;
    
    // Draw crosshair guides
    painter.line_segment(
        [Pos2::new(center.x - size, center.y), Pos2::new(center.x + size, center.y)],
        Stroke::new(0.5, styles::TEXT_SECONDARY),
    );
    painter.line_segment(
        [Pos2::new(center.x, center.y - size), Pos2::new(center.x, center.y + size)],
        Stroke::new(0.5, styles::TEXT_SECONDARY),
    );
    
    // Plot L vs R (using consecutive samples as pseudo-stereo for mono sources)
    let window = 512;
    let start = playhead.min(samples.len().saturating_sub(window));
    let chunk = &samples[start..start + window.min(samples.len() - start)];
    
    let mut points: Vec<Pos2> = Vec::with_capacity(chunk.len() / 2);
    for pair in chunk.chunks_exact(2) {
        let l = pair[0];
        let r = pair[1];
        let x = center.x + l * size;
        let y = center.y - r * size; // Invert Y for standard goniometer orientation
        points.push(Pos2::new(x, y));
    }
    
    if points.len() > 1 {
        // Draw as scattered dots via small line segments for density
        for pts in points.windows(2) {
            painter.line_segment([pts[0], pts[1]], Stroke::new(1.0, styles::PHASE_POSITIVE));
        }
    }
    
    painter.text(
        rect.left_top() + egui::vec2(10.0, 8.0),
        egui::Align2::LEFT_TOP,
        "PHASE",
        egui::FontId::proportional(9.0),
        styles::TEXT_SECONDARY,
    );
}

// =============================================================================
// 4. RMS / LUFS LOUDNESS METER
// =============================================================================
fn draw_rms_meter(painter: egui::Painter, rect: Rect, rms_db: f32, peak_db: f32) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);
    
    let margin = 15.0;
    let bar_left = rect.left() + margin;
    let bar_right = rect.right() - margin;
    let bar_top = rect.top() + 25.0;
    let bar_bottom = rect.bottom() - 10.0;
    let bar_height = bar_bottom - bar_top;
    
    // dB range: -60 to 0
    let min_db = -60.0_f32;
    let max_db = 0.0_f32;
    
    let rms_frac = ((rms_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
    let peak_frac = ((peak_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
    
    // Background track
    let track_rect = Rect::from_min_max(
        Pos2::new(bar_left, bar_top),
        Pos2::new(bar_right, bar_bottom),
    );
    painter.rect_filled(track_rect, Rounding::same(4.0), Color32::from_gray(20));
    
    // RMS fill bar (horizontal)
    let fill_width = rms_frac * track_rect.width();
    let fill_rect = Rect::from_min_size(
        Pos2::new(bar_left, bar_top),
        egui::vec2(fill_width, bar_height),
    );
    painter.rect_filled(fill_rect, Rounding::same(4.0), styles::RMS_BAR);
    
    // Peak hold marker (vertical line)
    let peak_x = bar_left + peak_frac * track_rect.width();
    painter.line_segment(
        [Pos2::new(peak_x, bar_top - 2.0), Pos2::new(peak_x, bar_bottom + 2.0)],
        Stroke::new(2.0, styles::PEAK_HOLD),
    );
    
    // dB labels
    painter.text(
        Pos2::new(bar_left, bar_bottom + 2.0),
        egui::Align2::LEFT_BOTTOM,
        "-60dB",
        egui::FontId::proportional(8.0),
        styles::TEXT_SECONDARY,
    );
    painter.text(
        Pos2::new(bar_right, bar_bottom + 2.0),
        egui::Align2::RIGHT_BOTTOM,
        "0dB",
        egui::FontId::proportional(8.0),
        styles::TEXT_SECONDARY,
    );
    
    // Current value readout
    painter.text(
        rect.right_top() + egui::vec2(-10.0, 8.0),
        egui::Align2::RIGHT_TOP,
        &format!("{:.1} dB", rms_db),
        egui::FontId::monospace(10.0),
        styles::TEXT_PRIMARY,
    );
    
    painter.text(
        rect.left_top() + egui::vec2(10.0, 8.0),
        egui::Align2::LEFT_TOP,
        "RMS LOUDNESS",
        egui::FontId::proportional(9.0),
        styles::TEXT_SECONDARY,
    );
}

// =============================================================================
// 5. OSCILLOSCOPE WITH PERSISTENCE
// =============================================================================
fn draw_persistence_oscilloscope(painter: egui::Painter, rect: Rect, buffer: &[Vec<Pos2>]) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);
    
    if buffer.is_empty() { return; }
    
    let width = rect.width() - 20.0;
    let start_x = rect.left() + 10.0;
    let mid_y = rect.center().y;
    
    // Draw older frames first with increasing opacity
    let total = buffer.len();
    for (frame_idx, points) in buffer.iter().enumerate() {
        let age = frame_idx as f32 / total as f32; // 0 = oldest, 1 = newest
        let alpha = (age * 200.0 + 30.0) as u8;
        let color = Color32::from_rgba_premultiplied(99, 179, 237, alpha);
        let stroke_width = 1.0 + age * 1.5;
        
        // Scale points to fit rect
        let scaled: Vec<Pos2> = points.iter().map(|p| {
            let x = start_x + (p.x / 512.0) * width; // Assuming 512 downsampled points
            let y = mid_y + p.y * (rect.height() / 120.0);
            Pos2::new(x.clamp(rect.left(), rect.right()), y.clamp(rect.top(), rect.bottom()))
        }).collect();
        
        if scaled.len() > 1 {
            painter.add(PathShape::line(scaled, Stroke::new(stroke_width, color)));
        }
    }
    
    painter.text(
        rect.left_top() + egui::vec2(10.0, 8.0),
        egui::Align2::LEFT_TOP,
        "OSCILLOSCOPE (PERSISTENCE)",
        egui::FontId::proportional(9.0),
        styles::TEXT_SECONDARY,
    );
}

// =============================================================================
// MINI PANELS (Bottom-right quadrant)
// =============================================================================
fn draw_waveform_mini(painter: egui::Painter, rect: Rect, samples: &[f32], playhead: usize) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);
    
    let mid_y = rect.center().y;
    let width = rect.width() - 20.0;
    let start_x = rect.left() + 10.0;
    
    let window_size = 1024;
    let start = playhead.min(samples.len().saturating_sub(window_size));
    let end = (start + window_size).min(samples.len());
    let window = &samples[start..end];
    
    if !window.is_empty() {
        let step = width / window.len() as f32;
        let points: Vec<Pos2> = window.iter().enumerate()
            .map(|(i, &s)| Pos2::new(start_x + i as f32 * step, mid_y - s * (rect.height() / 3.0)))
            .collect();
        
        if points.len() > 1 {
            painter.add(PathShape::line(points, Stroke::new(1.5, styles::WAVE_COLOR)));
        }
    }
    
    painter.text(
        rect.left_top() + egui::vec2(10.0, 6.0),
        egui::Align2::LEFT_TOP,
        "TIME DOMAIN",
        egui::FontId::proportional(9.0),
        styles::TEXT_SECONDARY,
    );
}

fn draw_spectrum_bars_mini(painter: egui::Painter, rect: Rect, spectrum: &[f32]) {
    painter.rect_filled(rect, Rounding::same(12.0), styles::PANEL_BG);
    
    let bar_count = 32;
    let usable_width = rect.width() - 20.0;
    let bar_width = usable_width / bar_count as f32;
    let gap = 2.0;
    let actual_w = (bar_width - gap).max(1.5);
    let start_x = rect.left() + 10.0;
    let bottom_y = rect.bottom() - 8.0;
    let max_h = rect.height() - 25.0;
    
    for i in 0..bar_count {
        let idx = (i as f32 / bar_count as f32 * spectrum.len() as f32) as usize;
        let mag = spectrum.get(idx).copied().unwrap_or(0.0);
        let adjusted = (mag * 80.0).powf(0.75).min(1.0);
        let h = adjusted * max_h;
        
        let x = start_x + i as f32 * bar_width;
        let color = if i < bar_count / 3 { styles::FREQ_LOW }
                    else if i < 2 * bar_count / 3 { styles::FREQ_MID }
                    else { styles::FREQ_HIGH };
        
        let bar_rect = Rect::from_min_max(
            Pos2::new(x, bottom_y - h),
            Pos2::new(x + actual_w, bottom_y),
        );
        painter.rect_filled(bar_rect, Rounding::same(2.0), color);
    }
    
    painter.text(
        rect.left_top() + egui::vec2(10.0, 6.0),
        egui::Align2::LEFT_TOP,
        "FREQUENCY",
        egui::FontId::proportional(9.0),
        styles::TEXT_SECONDARY,
    );
}

// =============================================================================
// EXISTING: Track Overview (unchanged from previous version)
// =============================================================================
fn draw_track_overview(ui: &mut egui::Ui, samples: &[f32], playhead: usize, total_samples: usize, height: f32) {
    let panel_rect = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(panel_rect.min, egui::vec2(panel_rect.width(), height));
    
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(8.0), styles::PANEL_BG);
    
    if total_samples == 0 { return; }
    
    let width = rect.width() - 20.0;
    let start_x = rect.left() + 10.0;
    let mid_y = rect.center().y;
    let step = (total_samples as f32 / width).max(1.0);
    let num_points = (width / 2.0) as usize;
    
    let mut points: Vec<Pos2> = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let sample_idx = ((i as f32 * step) as usize).min(total_samples - 1);
        let s = samples[sample_idx];
        let x = start_x + (i as f32 * 2.0);
        let y = mid_y - s * (height / 3.0);
        points.push(Pos2::new(x, y));
    }
    
    if points.len() > 1 {
        painter.add(PathShape::line(points, Stroke::new(1.0, styles::OVERVIEW_WAVE)));
    }
    
    let progress = playhead as f32 / total_samples as f32;
    let head_x = start_x + (progress * width);
    painter.line_segment(
        [Pos2::new(head_x, rect.top()), Pos2::new(head_x, rect.bottom())],
        Stroke::new(2.0, styles::PLAYHEAD_COLOR),
    );
    
    painter.text(
        rect.left_top() + egui::vec2(10.0, 5.0),
        egui::Align2::LEFT_TOP,
        "TRACK OVERVIEW",
        egui::FontId::proportional(9.0),
        styles::TEXT_SECONDARY,
    );
    
    ui.allocate_rect(rect, egui::Sense::hover());
}