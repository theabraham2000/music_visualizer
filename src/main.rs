mod app;
mod audio;
mod viz;

use eframe::NativeOptions;

fn main() -> Result<(), eframe::Error> {
    let options = NativeOptions::default();
    eframe::run_native(
        "MinSu Audio Station",
        options,
        Box::new(|cc| {
            Box::new(app::VisualizerApp::new(cc)) as Box<dyn eframe::App>
        }),
    )
}