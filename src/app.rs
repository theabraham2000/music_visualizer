use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::audio::{loader, engine::AudioEngine, processor::FftProcessor};
use crate::viz::renderer;
use crate::viz::styles;

pub struct VisualizerApp {
    samples: Arc<Mutex<Option<Vec<f32>>>>,
    audio_engine: AudioEngine,
    fft_processor: FftProcessor,
    spectrum: Vec<f32>,
    playhead: usize,
    is_playing: bool,
    current_track: String,
}

impl VisualizerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set light theme
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        
        Self {
            samples: Arc::new(Mutex::new(None)),
            audio_engine: AudioEngine::new(),
            fft_processor: FftProcessor::new(2048),
            spectrum: vec![0.0; 1024],
            playhead: 0,
            is_playing: false,
            current_track: "No track loaded".to_string(),
        }
    }

    fn load_track(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
                
            match loader::decode_mp3(path.clone()) {
                Ok(data) => {
                    *self.samples.lock().unwrap() = Some(data);
                    self.playhead = 0;
                    self.current_track = file_name;
                    
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
        // Custom top bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🎛️ MinSu Audio Station");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&self.current_track).color(styles::TEXT_SECONDARY));
                });
            });
        });
        
        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("● LIVE").color(styles::FREQ_LOW).strong());
                ui.separator();
                ui.label(format!("Sample Rate: 44.1kHz | FFT Size: 2048"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("v0.2.0-premium");
                });
            });
        });
        
        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                
                // Control panel
                ui.horizontal(|ui| {
                    if ui.button("📂 Load Track").clicked() {
                        self.load_track();
                    }
                    
                    if self.is_playing {
                        ui.label(egui::RichText::new("▶ Playing").color(styles::WAVE_COLOR));
                    } else {
                        ui.label(egui::RichText::new("⏸ Stopped").color(styles::TEXT_SECONDARY));
                    }
                });
                
                ui.add_space(20.0);
                
                // Visualizations
                let data_lock = self.samples.lock().unwrap();
                if let Some(samples) = &*data_lock {
                    let window_size = 2048;
                    let start = self.playhead.min(samples.len().saturating_sub(window_size));
                    let end = (start + window_size).min(samples.len());
                    if end - start == window_size {
                        self.spectrum = self.fft_processor.process(&samples[start..end]);
                    }
                    
                    renderer::draw_visualizer(ui, samples, &self.spectrum, self.playhead);
                    
                    if self.is_playing && !samples.is_empty() {
                        self.playhead = (self.playhead + 1024) % samples.len();
                    }
                } else {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("Load an MP3 to begin analysis")
                        .size(16.0)
                        .color(styles::TEXT_SECONDARY));
                }
            });
        });
        
        ctx.request_repaint();
    }
}