use eframe::egui::{self, Pos2, Rect, Stroke, Color32}; // Added Color32
use eframe::epaint::PathShape; // Removed RectShape
use super::styles;

pub fn draw_visualizer(ui: &mut egui::Ui, samples: &[f32], spectrum: &[f32], playhead: usize) {
    // Main container with padding
    ui.vertical_centered(|ui| {
        ui.add_space(10.0);
        
        // Waveform Panel
        draw_waveform_panel(ui, samples, playhead);
        
        ui.add_space(15.0);
        
        // Spectrum Panel
        draw_spectrum_panel(ui, spectrum);
        
        ui.add_space(10.0);
    });
}

fn draw_waveform_panel(ui: &mut egui::Ui, samples: &[f32], playhead: usize) {
    let panel_rect = ui.available_rect_before_wrap();
    let height = 180.0;
    let rect = Rect::from_min_size(panel_rect.min, egui::vec2(panel_rect.width(), height));
    
    // Draw panel background
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 12.0, styles::PANEL_BG);
    painter.rect_stroke(rect, 12.0, Stroke::new(1.0, styles::GRID_COLOR));
    
    // Draw subtle grid
    draw_grid(&painter, rect, 8, 4);
    
    // Draw waveform
    let mid_y = rect.center().y;
    let width = rect.width() - 40.0; // Padding
    let start_x = rect.left() + 20.0;
    
    let window_size = 2048;
    let start = playhead.min(samples.len().saturating_sub(window_size));
    let end = (start + window_size).min(samples.len());
    let window = &samples[start..end];
    
    if !window.is_empty() {
        let step = width / window.len() as f32;
        let points: Vec<Pos2> = window.iter().enumerate()
            .map(|(i, &s)| {
                let x = start_x + i as f32 * step;
                let y = mid_y - s * (height / 2.5);
                Pos2::new(x, y)
            })
            .collect();
        
        // Glow effect
        painter.add(PathShape::line(points.clone(), Stroke::new(6.0, styles::ACCENT_GLOW)));
        // Main line
        painter.add(PathShape::line(points, Stroke::new(2.5, styles::WAVE_COLOR)));
    }
    
    // Label
    painter.text(
        rect.left_top() + egui::vec2(15.0, 10.0),
        egui::Align2::LEFT_TOP,
        "TIME DOMAIN",
        egui::FontId::proportional(12.0),
        styles::TEXT_SECONDARY,
    );
    
    ui.allocate_rect(rect, egui::Sense::hover());
}

fn draw_spectrum_panel(ui: &mut egui::Ui, spectrum: &[f32]) {
    let panel_rect = ui.available_rect_before_wrap();
    let height = 200.0;
    let rect = Rect::from_min_size(panel_rect.min, egui::vec2(panel_rect.width(), height));
    
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 12.0, styles::PANEL_BG);
    painter.rect_stroke(rect, 12.0, Stroke::new(1.0, styles::GRID_COLOR));
    
    draw_grid(&painter, rect, 8, 5);
    
    let bar_count = 64; // Fewer, wider bars for premium look
    let usable_width = rect.width() - 40.0;
    let bar_width = usable_width / bar_count as f32;
    let gap = 2.0;
    let actual_bar_width = bar_width - gap;
    let start_x = rect.left() + 20.0;
    let bottom_y = rect.bottom() - 20.0;
    let max_height = height - 50.0;
    
    for i in 0..bar_count {
        // Map to spectrum (logarithmic feel)
        let idx = (i as f32 / bar_count as f32 * spectrum.len() as f32) as usize;
        let mag = spectrum.get(idx).copied().unwrap_or(0.0).min(1.0);
        
        // Apply gamma correction for better visibility
        let adjusted_mag = mag.powf(0.7);
        let bar_height = adjusted_mag * max_height;
        
        let x = start_x + i as f32 * bar_width;
        let y_top = bottom_y - bar_height;
        
        // Color based on frequency band
        let color = if i < bar_count / 3 {
            styles::FREQ_LOW
        } else if i < 2 * bar_count / 3 {
            styles::FREQ_MID
        } else {
            styles::FREQ_HIGH
        };
        
        // Draw bar with rounded top
        let bar_rect = Rect::from_min_max(
            Pos2::new(x, y_top),
            Pos2::new(x + actual_bar_width, bottom_y)
        );
        painter.rect_filled(bar_rect, 4.0, color);
        
        // Subtle reflection
        let reflect_rect = Rect::from_min_max(
            Pos2::new(x, bottom_y),
            Pos2::new(x + actual_bar_width, bottom_y + bar_height * 0.2)
        );
        painter.rect_filled(reflect_rect, 2.0, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 30));
    }
    
    // Labels
    painter.text(
        rect.left_top() + egui::vec2(15.0, 10.0),
        egui::Align2::LEFT_TOP,
        "FREQUENCY SPECTRUM",
        egui::FontId::proportional(12.0),
        styles::TEXT_SECONDARY,
    );
    
    painter.text(
        rect.left_bottom() + egui::vec2(15.0, -10.0),
        egui::Align2::LEFT_BOTTOM,
        "20Hz",
        egui::FontId::proportional(10.0),
        styles::TEXT_SECONDARY,
    );
    
    painter.text(
        rect.right_bottom() + egui::vec2(-15.0, -10.0),
        egui::Align2::RIGHT_BOTTOM,
        "20kHz",
        egui::FontId::proportional(10.0),
        styles::TEXT_SECONDARY,
    );
    
    ui.allocate_rect(rect, egui::Sense::hover());
}

fn draw_grid(painter: &egui::Painter, rect: Rect, cols: usize, rows: usize) {
    let width = rect.width();
    let height = rect.height();
    
    // Vertical lines
    for i in 1..cols {
        let x = rect.left() + (i as f32 / cols as f32) * width;
        painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], 
            Stroke::new(1.0, styles::GRID_COLOR));
    }
    
    // Horizontal lines
    for i in 1..rows {
        let y = rect.top() + (i as f32 / rows as f32) * height;
        painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], 
            Stroke::new(1.0, styles::GRID_COLOR));
    }
}