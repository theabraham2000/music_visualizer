use eframe::egui::{self, Pos2, Rect, Stroke, Color32, Rounding};
use eframe::epaint::{PathShape, Mesh};
use super::styles;

// UPDATED: Added total_samples parameter
pub fn draw_visualizer(ui: &mut egui::Ui, samples: &[f32], spectrum: &[f32], playhead: usize, total_samples: usize) {
    ui.vertical_centered(|ui| {
        let max_height = ui.available_height();
        
        // Allocate space for overview, waveform, and spectrum
        let overview_height = 60.0;
        let wave_height = (max_height * 0.40).min(200.0);
        let spec_height = (max_height * 0.40).min(220.0);
        
        // NEW: Draw full track overview at the top
        draw_track_overview(ui, samples, playhead, total_samples, overview_height);
        ui.add_space(10.0);
        
        draw_waveform_panel(ui, samples, playhead, wave_height);
        ui.add_space(20.0);
        draw_spectrum_panel(ui, spectrum, spec_height);
    });
}

/// Draws a miniaturized full-track waveform with a vertical playhead indicator
fn draw_track_overview(ui: &mut egui::Ui, samples: &[f32], playhead: usize, total_samples: usize, height: f32) {
    let panel_rect = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(panel_rect.min, egui::vec2(panel_rect.width(), height));
    
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(8.0), styles::PANEL_BG);
    
    if total_samples == 0 { return; }
    
    // Downsample for performance: one point per 2 pixels
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
    
    // Draw overview waveform
    if points.len() > 1 {
        painter.add(PathShape::line(points, Stroke::new(1.0, styles::OVERVIEW_WAVE)));
    }
    
    // Draw playhead bar
    let progress = playhead as f32 / total_samples as f32;
    let head_x = start_x + (progress * width);
    let head_line = PathShape::line(
        vec![Pos2::new(head_x, rect.top()), Pos2::new(head_x, rect.bottom())],
        Stroke::new(2.0, styles::PLAYHEAD_COLOR)
    );
    painter.add(head_line);
    
    // Label
    painter.text(
        rect.left_top() + egui::vec2(10.0, 5.0),
        egui::Align2::LEFT_TOP,
        "TRACK OVERVIEW",
        egui::FontId::proportional(9.0),
        styles::TEXT_SECONDARY,
    );
    
    ui.allocate_rect(rect, egui::Sense::hover());
}

fn draw_waveform_panel(ui: &mut egui::Ui, samples: &[f32], playhead: usize, height: f32) {
    let panel_rect = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(panel_rect.min, egui::vec2(panel_rect.width(), height));
    
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(16.0), styles::PANEL_BG);
    
    let mid_y = rect.center().y;
    let width = rect.width() - 60.0;
    let start_x = rect.left() + 30.0;
    
    let window_size = 2048;
    let start = playhead.min(samples.len().saturating_sub(window_size));
    let end = (start + window_size).min(samples.len());
    let window = &samples[start..end];
    
    if !window.is_empty() {
        let step = width / window.len() as f32;
        
        let mut points_top: Vec<Pos2> = Vec::with_capacity(window.len());
        let mut points_bottom: Vec<Pos2> = Vec::with_capacity(window.len());
        
        for (i, &s) in window.iter().enumerate() {
            let x = start_x + i as f32 * step;
            let y = mid_y - s * (height / 2.5);
            points_top.push(Pos2::new(x, y));
            points_bottom.push(Pos2::new(x, mid_y));
        }
        
        let mut mesh = Mesh::default();
        for i in 0..points_top.len().saturating_sub(1) {
            mesh.colored_vertex(points_top[i], styles::WAVE_FILL);
            mesh.colored_vertex(points_bottom[i], Color32::TRANSPARENT);
            mesh.colored_vertex(points_top[i+1], styles::WAVE_FILL);
            mesh.colored_vertex(points_bottom[i+1], Color32::TRANSPARENT);
            
            let idx = mesh.vertices.len() as u32 - 4;
            mesh.add_triangle(idx, idx+1, idx+2);
            mesh.add_triangle(idx+1, idx+3, idx+2);
        }
        painter.add(mesh);
        painter.add(PathShape::line(points_top, Stroke::new(2.0, styles::WAVE_COLOR)));
    }
    
    painter.text(
        rect.left_top() + egui::vec2(20.0, 15.0),
        egui::Align2::LEFT_TOP,
        "TIME DOMAIN",
        egui::FontId::proportional(11.0), 
        styles::TEXT_SECONDARY,
    );
    
    ui.allocate_rect(rect, egui::Sense::hover());
}

fn draw_spectrum_panel(ui: &mut egui::Ui, spectrum: &[f32], height: f32) {
    let panel_rect = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(panel_rect.min, egui::vec2(panel_rect.width(), height));
    
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(16.0), styles::PANEL_BG);
    
    let bar_count = 64;
    let usable_width = rect.width() - 60.0;
    let bar_width = usable_width / bar_count as f32;
    let gap = 3.0;
    let actual_bar_width = (bar_width - gap).max(2.0);
    let start_x = rect.left() + 30.0;
    
    let bottom_y = rect.bottom() - 30.0;
    let max_bar_height = height - 60.0;
    
    for i in 0..bar_count {
        let idx = (i as f32 / bar_count as f32 * spectrum.len() as f32) as usize;
        let mag = spectrum.get(idx).copied().unwrap_or(0.0);
        
        let adjusted_mag = (mag * 80.0).powf(0.75).min(1.0);
        let bar_height = adjusted_mag * max_bar_height;
        
        let x = start_x + i as f32 * bar_width;
        let y_top = bottom_y - bar_height;
        
        let base_color = if i < bar_count / 3 {
            styles::FREQ_LOW
        } else if i < 2 * bar_count / 3 {
            styles::FREQ_MID
        } else {
            styles::FREQ_HIGH
        };
        
        let bar_rect = Rect::from_min_max(
            Pos2::new(x, y_top),
            Pos2::new(x + actual_bar_width, bottom_y)
        );
        
        painter.rect_filled(bar_rect, Rounding::same(4.0), base_color);
        
        if bar_height > 1.0 {
            let reflect_h = (bar_height * 0.2).min(15.0);
            let reflect_rect = Rect::from_min_max(
                Pos2::new(x, bottom_y),
                Pos2::new(x + actual_bar_width, bottom_y + reflect_h)
            );
            painter.rect_filled(reflect_rect, Rounding::same(2.0), 
                Color32::from_rgba_premultiplied(base_color.r(), base_color.g(), base_color.b(), 20));
        }
    }
    
    painter.text(
        rect.left_top() + egui::vec2(20.0, 15.0),
        egui::Align2::LEFT_TOP,
        "FREQUENCY SPECTRUM",
        egui::FontId::proportional(11.0),
        styles::TEXT_SECONDARY,
    );
    
    painter.text(
        Pos2::new(rect.left() + 20.0, bottom_y + 18.0),
        egui::Align2::LEFT_CENTER,
        "20Hz",
        egui::FontId::proportional(10.0),
        styles::TEXT_SECONDARY,
    );
    
    painter.text(
        Pos2::new(rect.right() - 20.0, bottom_y + 18.0),
        egui::Align2::RIGHT_CENTER,
        "20kHz",
        egui::FontId::proportional(10.0),
        styles::TEXT_SECONDARY,
    );
    
    ui.allocate_rect(rect, egui::Sense::hover());
}