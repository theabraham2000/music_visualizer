use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::audio::{loader, engine::AudioEngine, processor::FftProcessor};
use crate::viz::renderer;
use crate::viz::styles;

const SAMPLE_RATE: u32 = 44100;
const FFT_SIZE: usize = 2048;
// Assuming ~60 FPS, advance playhead by this many samples per frame
const SAMPLES_PER_FRAME: usize = (SAMPLE_RATE / 60) as usize;

pub struct VisualizerApp {
    samples: Arc<Mutex<Option<Vec<f32>>>>,
    audio_engine: AudioEngine,
    fft_processor: FftProcessor,
    spectrum: Vec<f32>,
    playhead: usize,
    current_track: String,
}

impl VisualizerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        
        Self {
            samples: Arc::new(Mutex::new(None)),
            audio_engine: AudioEngine::new(),
            fft_processor: FftProcessor::new(FFT_SIZE),
            spectrum: vec![0.0; FFT_SIZE / 2],
            playhead: 0,
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
                    
                    if self.audio_engine.load_and_play(path).is_ok() {
                        // Playing started successfully
                    } else {
                        eprintln!("Failed to start audio playback");
                    }
                }
                Err(e) => eprintln!("Error loading audio: {}", e),
            }
        }
    }
}

impl eframe::App for VisualizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🎛️ MinSu Audio Station");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&self.current_track)
                        .color(styles::TEXT_SECONDARY)
                        .size(14.0));
                });
            });
        });
        
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let status_color = if self.audio_engine.is_playing() { 
                    styles::FREQ_LOW 
                } else if self.audio_engine.is_loaded() { 
                    styles::FREQ_MID 
                } else { 
                    styles::TEXT_SECONDARY 
                };
                
                ui.label(egui::RichText::new("● LIVE").color(status_color).strong());
                ui.separator();
                ui.label(format!("Sample Rate: {}kHz | FFT Size: {}", SAMPLE_RATE/1000, FFT_SIZE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("v0.2.1-synced");
                });
            });
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(15.0);
                
                ui.horizontal(|ui| {
                    if ui.button("📂 Load Track").clicked() {
                        self.load_track();
                    }
                    
                    ui.add_space(10.0);
                    
                    let is_loaded = self.audio_engine.is_loaded();
                    let is_playing = self.audio_engine.is_playing();
                    
                    ui.add_enabled(is_loaded, egui::Button::new(if is_playing { "⏸ Pause" } else { "▶ Play" }))
                      .on_disabled_hover_text("Load a track first");
                      
                    if ui.add_enabled(is_loaded, egui::Button::new("⏹ Stop")).clicked() && is_loaded {
                        self.audio_engine.stop();
                        self.playhead = 0;
                    }
                    
                    ui.add_space(10.0);
                    
                    let status_text = if is_playing { "Playing" } 
                                      else if is_loaded { "Paused" } 
                                      else { "Ready" };
                                      
                    let status_color = if is_playing { styles::WAVE_COLOR } 
                                       else if is_loaded { styles::FREQ_MID } 
                                       else { styles::TEXT_SECONDARY };
                                       
                    ui.label(egui::RichText::new(status_text).color(status_color).strong());
                });
                
                ui.add_space(20.0);
                
                let data_lock = self.samples.lock().unwrap();
                if let Some(samples) = &*data_lock {
                    // FRAME-BASED SYNC: Advance playhead deterministically
                    if self.audio_engine.is_playing() {
                        self.playhead = (self.playhead + SAMPLES_PER_FRAME) % samples.len();
                    }
                    
                    // Safe FFT processing with bounds check
                    if samples.len() >= FFT_SIZE {
                        let safe_start = self.playhead.min(samples.len() - FFT_SIZE);
                        self.spectrum = self.fft_processor.process(&samples[safe_start..safe_start + FFT_SIZE]);
                    }
                    
                    renderer::draw_visualizer(ui, samples, &self.spectrum, self.playhead);
                } else {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("Load an MP3 to begin analysis")
                        .size(16.0)
                        .color(styles::TEXT_SECONDARY));
                }
            });
        });
        
        if self.audio_engine.is_playing() {
            ctx.request_repaint();
        }
    }
}