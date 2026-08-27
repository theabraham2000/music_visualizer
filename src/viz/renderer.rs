use eframe::egui::{self, Pos2, Rect, Stroke, Color32, Rounding};
use eframe::epaint::{PathShape, Mesh}; // Removed TessellationOptions
use super::styles;

pub fn draw_visualizer(ui: &mut egui::Ui, samples: &[f32], spectrum: &[f32], playhead: usize) {
    ui.vertical_centered(|ui| {
        let max_height = ui.available_height();
        let wave_height = (max_height * 0.45).min(220.0);
        let spec_height = (max_height * 0.45).min(240.0);
        
        draw_waveform_panel(ui, samples, playhead, wave_height);
        ui.add_space(20.0);
        draw_spectrum_panel(ui, spectrum, spec_height);
    });
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
        
        // Filled Area Mesh
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
        
        // Top Line
        painter.add(PathShape::line(points_top, Stroke::new(2.0, styles::WAVE_COLOR)));
    }
    
    // FIXED: Removed .spacing()
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
    
    // FIXED: Removed .spacing()
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