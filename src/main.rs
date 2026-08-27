mod app;
mod audio;
mod viz;

use eframe::NativeOptions;

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions::default();
    eframe::run_native(
        "Music Visualizer - MinSu Lab",
        options,
        Box::new(|_cc| {
            Box::new(app::VisualizerApp::new()) as Box<dyn eframe::App>
        }),
    )
}