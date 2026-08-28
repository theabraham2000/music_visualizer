mod app;
mod audio;
mod viz;
mod utils;

use eframe::NativeOptions;
use eframe::egui::ViewportBuilder;

fn main() -> Result<(), eframe::Error> {
    let viewport = ViewportBuilder::default()
        .with_inner_size([960.0, 640.0])
        .with_min_inner_size([700.0, 500.0])
        .with_title("MinSu Music Visualizer");

    let options = NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "MinSu Music Visualizer",
        options,
        Box::new(|cc| {
            Box::new(app::VisualizerApp::new(cc)) as Box<dyn eframe::App>
        }),
    )
}