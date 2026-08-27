use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::audio::{loader, engine::AudioEngine, processor::FftProcessor};
use crate::viz::renderer;
use crate::viz::styles;

const SAMPLE_RATE: u32 = 44100;
const FFT_SIZE: usize = 2048;
const SAMPLES_PER_FRAME: usize = (SAMPLE_RATE / 60) as usize;

pub struct VisualizerApp {
    samples: Arc<Mutex<Option<Vec<f32>>>>,
    audio_engine: AudioEngine,
    fft_processor: FftProcessor,
    spectrum: Vec<f32>,
    smoothed_spectrum: Vec<f32>, // For visual smoothing
    playhead: usize,
    current_track: String,
}

impl VisualizerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark()); // Force dark mode
        
        Self {
            samples: Arc::new(Mutex::new(None)),
            audio_engine: AudioEngine::new(),
            fft_processor: FftProcessor::new(FFT_SIZE),
            spectrum: vec![0.0; FFT_SIZE / 2],
            smoothed_spectrum: vec![0.0; FFT_SIZE / 2],
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
                    self.smoothed_spectrum.fill(0.0);
                    
                    if let Err(e) = self.audio_engine.load(path) {
                        eprintln!("Audio Error: {}", e);
                    }
                }
                Err(e) => eprintln!("Decode Error: {}", e),
            }
        }
    }
}

impl eframe::App for VisualizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // TOP BAR
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("🎛️ MinSu Audio Station").color(styles::TEXT_PRIMARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&self.current_track)
                        .color(styles::TEXT_SECONDARY)
                        .size(14.0)
                        .monospace());
                });
            });
        });
        
        // STATUS BAR
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let status_color = if self.audio_engine.is_playing() { 
                    styles::WAVE_COLOR 
                } else { 
                    styles::TEXT_SECONDARY 
                };
                
                ui.label(egui::RichText::new("● LIVE").color(status_color).strong());
                ui.separator();
                ui.label(egui::RichText::new(format!("SR: {}k | FFT: {}", SAMPLE_RATE/1000, FFT_SIZE))
                    .color(styles::TEXT_SECONDARY)
                    .size(12.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("v0.3.0-pro").color(styles::TEXT_SECONDARY).size(12.0));
                });
            });
        });
        
        // CENTRAL PANEL
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().visuals.widgets.inactive.bg_fill = styles::BG_PRIMARY;
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = styles::BG_PRIMARY;
            
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                
                // CONTROLS
                ui.horizontal(|ui| {
                    ui.visuals_mut().button_frame = true;
                    
                    if ui.button("📂 Load Track").clicked() {
                        self.load_track();
                    }
                    
                    ui.add_space(15.0);
                    
                    let has_track = self.audio_engine.has_track();
                    let is_playing = self.audio_engine.is_playing();
                    
                    // Play/Pause Button
                    let btn_text = if is_playing { " Pause" } else { "▶ Play" };
                    if ui.add_enabled(has_track, egui::Button::new(btn_text)).clicked() && has_track {
                        if let Err(e) = self.audio_engine.toggle_play_pause() {
                            eprintln!("Control Error: {}", e);
                        }
                    }
                    
                    // Stop Button
                    if ui.add_enabled(has_track, egui::Button::new("⏹ Stop")).clicked() && has_track {
                        self.audio_engine.stop();
                        self.playhead = 0;
                        self.smoothed_spectrum.fill(0.0); // Clear visuals immediately
                    }
                    
                    ui.add_space(15.0);
                    
                    let status_text = if is_playing { "Playing" } 
                                      else if has_track { "Paused" } 
                                      else { "Ready" };
                                      
                    let status_color = if is_playing { styles::WAVE_COLOR } 
                                       else if has_track { styles::FREQ_MID } 
                                       else { styles::TEXT_SECONDARY };
                                       
                    ui.label(egui::RichText::new(status_text).color(status_color).strong().size(14.0));
                });
                
                ui.add_space(25.0);
                
                // VISUALIZATION
                let data_lock = self.samples.lock().unwrap();
                if let Some(samples) = &*data_lock {
                    // SYNC LOGIC
                    if self.audio_engine.is_playing() {
                        self.playhead = (self.playhead + SAMPLES_PER_FRAME) % samples.len();
                    }
                    
                    // PROCESS FFT
                    if samples.len() >= FFT_SIZE && (self.audio_engine.is_playing() || !self.smoothed_spectrum.iter().all(|&x| x == 0.0)) {
                        let safe_start = self.playhead.min(samples.len() - FFT_SIZE);
                        let raw_spectrum = self.fft_processor.process(&samples[safe_start..safe_start + FFT_SIZE]);
                        
                        // SMOOTHING (Exponential Moving Average)
                        for i in 0..self.smoothed_spectrum.len() {
                            let target = raw_spectrum[i];
                            // 0.3 = fast response, 0.1 = slow/smooth
                            self.smoothed_spectrum[i] += (target - self.smoothed_spectrum[i]) * 0.4;
                        }
                    } else if !self.audio_engine.is_playing() && self.audio_engine.has_track() {
                         // Optional: Decay spectrum when paused/stopped for cool effect
                         for val in self.smoothed_spectrum.iter_mut() {
                             *val *= 0.9;
                         }
                    }
                    
                    renderer::draw_visualizer(ui, samples, &self.smoothed_spectrum, self.playhead);
                } else {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("Load an MP3 to begin analysis")
                        .size(18.0)
                        .color(styles::TEXT_SECONDARY));
                }
            });
        });
        
        if self.audio_engine.is_playing() || self.smoothed_spectrum.iter().any(|&x| x > 0.01) {
            ctx.request_repaint();
        }
    }
}