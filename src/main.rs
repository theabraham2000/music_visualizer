mod app;

use eframe::NativeOptions;

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions::default();
    eframe::run_native(
        "Music Visualizer - MinSu Lab",
        options,
        Box::new(|_cc| Ok(Box::new(app::VisualizerApp::new()))),
    )
}