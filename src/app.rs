use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::audio::{loader, engine::AudioEngine, processor::FftProcessor};
use crate::viz::renderer;
use crate::viz::styles;

const SAMPLE_RATE: u32 = 44100;
const FFT_SIZE: usize = 2048;
const SAMPLES_PER_FRAME: usize = (SAMPLE_RATE / 60) as usize;
const SPECTROGRAM_WIDTH: usize = 300; // Columns of history to keep
const PERSISTENCE_FRAMES: usize = 8;  // How many frames to trail

pub struct VisualizerApp {
    samples: Arc<Mutex<Option<Vec<f32>>>>,
    audio_engine: AudioEngine,
    fft_processor: FftProcessor,
    smoothed_spectrum: Vec<f32>,
    playhead: usize,
    current_track: String,
    
    // NEW: Panel-specific state
    spectrogram_history: Vec<Vec<f32>>, // Ring buffer of FFT columns
    persistence_buffer: Vec<Vec<egui::Pos2>>, // Last N frames of waveform points
    rms_smoothed: f32,
    peak_hold: f32,
    peak_decay_timer: f32,
    radial_rotation: f32,
}

impl VisualizerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        
        Self {
            samples: Arc::new(Mutex::new(None)),
            audio_engine: AudioEngine::new(),
            fft_processor: FftProcessor::new(FFT_SIZE),
            smoothed_spectrum: vec![0.0; FFT_SIZE / 2],
            playhead: 0,
            current_track: "No track loaded".to_string(),
            
            spectrogram_history: Vec::with_capacity(SPECTROGRAM_WIDTH),
            persistence_buffer: Vec::with_capacity(PERSISTENCE_FRAMES),
            rms_smoothed: 0.0,
            peak_hold: 0.0,
            peak_decay_timer: 0.0,
            radial_rotation: 0.0,
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
                    
                    // Reset panel states on new track
                    self.spectrogram_history.clear();
                    self.persistence_buffer.clear();
                    self.rms_smoothed = 0.0;
                    self.peak_hold = 0.0;
                    self.peak_decay_timer = 0.0;
                    self.radial_rotation = 0.0;
                    
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
        self.audio_engine.check_finished();
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("MinSu Audio Station").color(styles::TEXT_PRIMARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&self.current_track)
                        .color(styles::TEXT_SECONDARY)
                        .size(14.0)
                        .monospace());
                });
            });
        });
        
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let status_color = if self.audio_engine.is_playing() { 
                    styles::WAVE_COLOR 
                } else { 
                    styles::TEXT_SECONDARY 
                };
                ui.label(egui::RichText::new("[LIVE]").color(status_color).strong());
                ui.separator();
                ui.label(egui::RichText::new(format!("SR: {}k | FFT: {}", SAMPLE_RATE/1000, FFT_SIZE))
                    .color(styles::TEXT_SECONDARY)
                    .size(12.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("v0.4.0-stable").color(styles::TEXT_SECONDARY).size(12.0));
                });
            });
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().visuals.widgets.inactive.bg_fill = styles::BG_PRIMARY;
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = styles::BG_PRIMARY;
            
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.visuals_mut().button_frame = true;
                    if ui.button("Load Track").clicked() { self.load_track(); }
                    ui.add_space(15.0);
                    
                    let has_track = self.audio_engine.has_track();
                    let is_playing = self.audio_engine.is_playing();
                    
                    let btn_text = if is_playing { "Pause" } else { "Play" };
                    if ui.add_enabled(has_track, egui::Button::new(btn_text)).clicked() && has_track {
                        if let Err(e) = self.audio_engine.toggle_play_pause() {
                            eprintln!("Control Error: {}", e);
                        }
                        if !self.audio_engine.is_playing() {
                            self.smoothed_spectrum.fill(0.0);
                        }
                    }
                    
                    if ui.add_enabled(has_track, egui::Button::new("Stop")).clicked() && has_track {
                        self.audio_engine.stop();
                        self.playhead = 0;
                        self.smoothed_spectrum.fill(0.0);
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
                
                ui.add_space(15.0);
                
                let data_lock = self.samples.lock().unwrap();
                if let Some(samples) = &*data_lock {
                    let is_playing = self.audio_engine.is_playing();
                    
                    if is_playing {
                        self.playhead = (self.playhead + SAMPLES_PER_FRAME) % samples.len();
                    }
                    
                    let has_energy = self.smoothed_spectrum.iter().any(|&x| x > 0.001);
                    
                    if samples.len() >= FFT_SIZE && (is_playing || has_energy) {
                        let safe_start = self.playhead.min(samples.len() - FFT_SIZE);
                        let window = &samples[safe_start..safe_start + FFT_SIZE];
                        let raw_spectrum = self.fft_processor.process(window);
                        
                        // Update smoothed spectrum
                        if is_playing {
                            for i in 0..self.smoothed_spectrum.len() {
                                let target = raw_spectrum[i];
                                self.smoothed_spectrum[i] += (target - self.smoothed_spectrum[i]) * 0.4;
                            }
                        } else {
                            for val in self.smoothed_spectrum.iter_mut() {
                                *val *= 0.5;
                                if *val < 0.001 { *val = 0.0; }
                            }
                        }
                        
                        // UPDATE: Spectrogram History (push new column, pop oldest)
                        if is_playing {
                            self.spectrogram_history.push(raw_spectrum.clone());
                            if self.spectrogram_history.len() > SPECTROGRAM_WIDTH {
                                self.spectrogram_history.remove(0);
                            }
                        }
                        
                        // UPDATE: RMS / Peak Hold Calculation
                        let rms = (window.iter().map(|s| s * s).sum::<f32>() / window.len() as f32).sqrt();
                        let db = if rms > 0.0 { 20.0 * rms.log10() } else { -60.0 };
                        self.rms_smoothed += (db - self.rms_smoothed) * 0.15;
                        
                        if db > self.peak_hold {
                            self.peak_hold = db;
                            self.peak_decay_timer = 1.0; // Hold for ~1 second at 60fps
                        } else if self.peak_decay_timer > 0.0 {
                            self.peak_decay_timer -= 1.0 / 60.0;
                        } else {
                            self.peak_hold += (db - self.peak_hold) * 0.05; // Slow decay after hold
                        }
                        
                        // UPDATE: Persistence Oscilloscope Buffer
                        if is_playing {
                            let mid_y = 0.0; // Will be offset in renderer
                            let step = 1.0;
                            let points: Vec<egui::Pos2> = window.iter().enumerate()
                                .step_by(4) // Downsample for performance
                                .map(|(i, &s)| egui::Pos2::new(i as f32 * step, mid_y - s * 50.0))
                                .collect();
                            self.persistence_buffer.push(points);
                            if self.persistence_buffer.len() > PERSISTENCE_FRAMES {
                                self.persistence_buffer.remove(0);
                            }
                        }
                        
                        // UPDATE: Radial Rotation
                        if is_playing {
                            self.radial_rotation += 0.005;
                        }
                    } else if !is_playing && !has_energy {
                        self.smoothed_spectrum.fill(0.0);
                    }
                    
                    renderer::draw_visualizer(
                        ui, 
                        samples, 
                        &self.smoothed_spectrum, 
                        self.playhead, 
                        samples.len(),
                        &self.spectrogram_history,
                        &self.persistence_buffer,
                        self.rms_smoothed,
                        self.peak_hold,
                        self.radial_rotation,
                    );
                } else {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("Load an MP3 to begin analysis")
                        .size(18.0)
                        .color(styles::TEXT_SECONDARY));
                }
            });
        });
        
        let needs_repaint = self.audio_engine.is_playing() 
            || self.smoothed_spectrum.iter().any(|&x| x > 0.001);
        if needs_repaint {
            ctx.request_repaint();
        }
    }
}