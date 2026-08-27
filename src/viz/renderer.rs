use eframe::egui;
use eframe::epaint::Stroke;
use super::styles;

pub fn draw_visualizer(ui: &mut egui::Ui, samples: &[f32], spectrum: &[f32], playhead: usize) {
    // Top half: Waveform
    let (_resp1, rect1) = ui.allocate_space(egui::vec2(ui.available_width(), 150.0));
    draw_waveform(ui.ctx(), rect1, samples, playhead);

    ui.separator();

    // Bottom half: Frequency Spectrum
    let (_resp2, rect2) = ui.allocate_space(egui::vec2(ui.available_width(), 150.0));
    draw_spectrum(ui.ctx(), rect2, spectrum);
}

fn draw_waveform(ctx: &egui::Context, rect: egui::Rect, samples: &[f32], playhead: usize) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let width = rect.width();
    let height = rect.height();
    let mid_y = rect.center().y;

    let window_size = 2048;
    let start = playhead.min(samples.len().saturating_sub(window_size));
    let end = (start + window_size).min(samples.len());
    let window = &samples[start..end];
    if window.is_empty() { return; }

    let step = width / window.len() as f32;
    let path: Vec<egui::Pos2> = window.iter().enumerate()
        .map(|(i, &s)| egui::Pos2::new(rect.left() + i as f32 * step, mid_y - s * height / 2.0))
        .collect();

    painter.add(egui::Shape::line(path, Stroke::new(2.0, styles::WAVE_COLOR)));
}

fn draw_spectrum(ctx: &egui::Context, rect: egui::Rect, spectrum: &[f32]) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let width = rect.width();
    let height = rect.height();
    let bar_count = spectrum.len().min(256); // Limit bars for performance
    let bar_width = width / bar_count as f32;

    for i in 0..bar_count {
        let mag = spectrum[i].min(1.0); // Clamp
        let bar_height = mag * height;
        let x = rect.left() + i as f32 * bar_width;
        let y_top = rect.bottom() - bar_height;

        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(x, y_top), egui::pos2(x + bar_width - 1.0, rect.bottom())),
            0.0,
            styles::FREQ_COLOR,
        );
    }
}