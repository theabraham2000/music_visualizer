use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::audio::loader;
use crate::viz::renderer;

pub struct VisualizerApp {
    samples: Arc<Mutex<Option<Vec<f32>>>>,
    playhead: usize,
    is_playing: bool,
}

impl VisualizerApp {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(None)),
            playhead: 0,
            is_playing: false,
        }
    }

    fn load_track(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            match loader::decode_mp3(path) {
                Ok(data) => {
                    *self.samples.lock().unwrap() = Some(data);
                    self.is_playing = true;
                    self.playhead = 0;
                }
                Err(e) => eprintln!("Error loading audio: {}", e),
            }
        }
    }
}

impl eframe::App for VisualizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🍂 Music Visualizer");
            
            if ui.button("📂 Load MP3").clicked() {
                self.load_track();
            }

            ui.separator();

            let data_lock = self.samples.lock().unwrap();
            if let Some(samples) = &*data_lock {
                renderer::draw_waveform(ui, samples, self.playhead);
                
                // Simulate playback movement
                if self.is_playing && !samples.is_empty() {
                    self.playhead = (self.playhead + 512) % samples.len();
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Ready for vibes. Load an MP3 to begin.");
                });
            }
        });
        ctx.request_repaint();
    }
}