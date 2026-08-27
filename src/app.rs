use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::audio::{loader, engine::AudioEngine, processor::FftProcessor};
use crate::viz::renderer;

pub struct VisualizerApp {
    samples: Arc<Mutex<Option<Vec<f32>>>>,
    audio_engine: AudioEngine,
    fft_processor: FftProcessor,
    spectrum: Vec<f32>,
    playhead: usize,
    is_playing: bool,
}

impl VisualizerApp {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(None)),
            audio_engine: AudioEngine::new(),
            fft_processor: FftProcessor::new(2048),
            spectrum: vec![0.0; 1024],
            playhead: 0,
            is_playing: false,
        }
    }

    fn load_track(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            match loader::decode_mp3(path.clone()) {
                Ok(data) => {
                    *self.samples.lock().unwrap() = Some(data);
                    self.playhead = 0;
                    
                    // Start real audio playback
                    if self.audio_engine.play(path).is_ok() {
                        self.is_playing = true;
                    } else {
                        eprintln!("Failed to start audio playback");
                        self.is_playing = false;
                    }
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
                // Update FFT on current window
                let window_size = 2048;
                let start = self.playhead.min(samples.len().saturating_sub(window_size));
                let end = (start + window_size).min(samples.len());
                if end - start == window_size {
                    self.spectrum = self.fft_processor.process(&samples[start..end]);
                }

                renderer::draw_visualizer(ui, samples, &self.spectrum, self.playhead);
                
                // Advance playhead (~44.1kHz sample rate, advance ~1024 samples per frame)
                if self.is_playing && !samples.is_empty() {
                    self.playhead = (self.playhead + 1024) % samples.len();
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