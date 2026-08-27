use eframe::egui;
use eframe::epaint::Stroke;
use super::styles;

pub fn draw_waveform(ui: &mut egui::Ui, samples: &[f32], playhead: usize) {
    let (_response, rect) = ui.allocate_space(egui::vec2(ui.available_width(), 300.0));
    
    let painter = ui.ctx().layer_painter(egui::LayerId::background());
    
    let width = rect.width();
    let height = rect.height();
    let mid_y = rect.center().y;

    let window_size = 2048;
    let start = playhead.min(samples.len().saturating_sub(window_size));
    let end = (start + window_size).min(samples.len());
    let window = &samples[start..end];

    if window.is_empty() { return; }

    let step = width / window.len() as f32;
    let mut path = vec![];

    for (i, &sample) in window.iter().enumerate() {
        let x = rect.left() + i as f32 * step;
        let y = mid_y - (sample * height / 2.0);
        path.push(egui::Pos2::new(x, y));
    }

    // Create a Stroke with width and color
    let stroke = Stroke::new(2.0, styles::WAVE_COLOR);
    painter.add(egui::Shape::line(path, stroke));
}