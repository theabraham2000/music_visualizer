use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::audio::{loader, engine::AudioEngine, processor::FftProcessor};
use crate::viz::renderer;
use crate::viz::styles;

const FFT_SIZE: usize = 2048;
const SPECTROGRAM_WIDTH: usize = 400;
const PERSISTENCE_FRAMES: usize = 8;

pub struct VisualizerApp {
    audio_data: Arc<Mutex<Option<loader::AudioData>>>,
    audio_engine: AudioEngine,
    fft_processor: FftProcessor,
    smoothed_spectrum: Vec<f32>,
    current_track: String,

    spectrogram_history: Vec<Vec<f32>>,
    persistence_buffer: Vec<Vec<egui::Pos2>>,
    rms_smoothed: f32,
    peak_hold: f32,
    peak_decay_timer: f32,
    radial_rotation: f32,

    // Precomputed min/max envelope for track overview
    overview_envelope: Vec<(f32, f32)>,
}

impl VisualizerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        Self {
            audio_data: Arc::new(Mutex::new(None)),
            audio_engine: AudioEngine::new(),
            fft_processor: FftProcessor::new(FFT_SIZE),
            smoothed_spectrum: vec![0.0; FFT_SIZE / 2],
            current_track: "No track loaded".to_string(),
            spectrogram_history: Vec::with_capacity(SPECTROGRAM_WIDTH),
            persistence_buffer: Vec::with_capacity(PERSISTENCE_FRAMES),
            rms_smoothed: -60.0,
            peak_hold: -60.0,
            peak_decay_timer: 0.0,
            radial_rotation: 0.0,
            overview_envelope: Vec::new(),
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
                    // Build min/max envelope for overview waveform
                    let pixels = 800;
                    let samples_per_pixel = (data.samples.len() / pixels).max(1);
                    let mut envelope = Vec::with_capacity(pixels);
                    for chunk in data.samples.chunks(samples_per_pixel) {
                        let min = chunk.iter().cloned().fold(f32::INFINITY, f32::min);
                        let max = chunk.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                        envelope.push((min, max));
                    }
                    self.overview_envelope = envelope;

                    *self.audio_data.lock().unwrap() = Some(data);
                    self.current_track = file_name;
                    self.smoothed_spectrum.fill(0.0);
                    self.spectrogram_history.clear();
                    self.persistence_buffer.clear();
                    self.rms_smoothed = -60.0;
                    self.peak_hold = -60.0;
                    self.peak_decay_timer = 0.0;
                    self.radial_rotation = 0.0;

                    let dur = self.audio_data.lock().unwrap().as_ref().unwrap().duration_secs;
                    if let Err(e) = self.audio_engine.load(path, dur) {
                        eprintln!("Audio Error: {}", e);
                    }
                }
                Err(e) => eprintln!("Decode Error: {}", e),
            }
        }
    }

    /// Extract properly separated L/R channels from interleaved buffer
    fn get_stereo_window(data: &loader::AudioData, playhead: usize, window_size: usize) -> (Vec<f32>, Vec<f32>) {
        let total_frames = data.samples.len() / data.channels;
        let start_frame = playhead.min(total_frames.saturating_sub(window_size));
        let end_frame = (start_frame + window_size).min(total_frames);

        let mut left = Vec::with_capacity(end_frame - start_frame);
        let mut right = Vec::with_capacity(end_frame - start_frame);

        if data.channels == 2 {
            for frame in start_frame..end_frame {
                left.push(data.samples[frame * 2]);
                right.push(data.samples[frame * 2 + 1]);
            }
        } else {
            // Mono: duplicate to both channels
            for frame in start_frame..end_frame {
                let s = data.samples[frame];
                left.push(s);
                right.push(s);
            }
        }
        (left, right)
    }
}

impl eframe::App for VisualizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.audio_engine.check_finished();

        // === TOP BAR ===
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("MINSU AUDIO STATION")
                    .color(styles::TEXT_PRIMARY)
                    .size(16.0)
                    .strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let live_color = if self.audio_engine.is_playing() {
                        styles::WAVE_COLOR
                    } else {
                        styles::TEXT_SECONDARY
                    };
                    ui.label(egui::RichText::new("LIVE").color(live_color).strong().size(12.0));
                });
            });

            ui.add_space(4.0);
            ui.separator();

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&self.current_track)
                    .color(styles::TEXT_PRIMARY)
                    .size(13.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let pos = self.audio_engine.position_seconds();
                    let dur = self.audio_engine.duration_seconds();
                    let fmt_time = |t: f32| -> String {
                        let m = (t / 60.0) as u32;
                        let s = (t % 60.0) as u32;
                        format!("{:02}:{:02}", m, s)
                    };
                    ui.label(egui::RichText::new(format!("{} / {}", fmt_time(pos), fmt_time(dur)))
                        .color(styles::TEXT_SECONDARY)
                        .monospace()
                        .size(12.0));
                });
            });
            ui.add_space(4.0);
        });

        // === BOTTOM CONTROLS ===
        egui::TopBottomPanel::bottom("controls").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal_centered(|ui| {
                ui.visuals_mut().button_frame = true;

                if ui.button("Load").clicked() {
                    self.load_track();
                }
                ui.add_space(10.0);

                let has_track = self.audio_engine.has_track();
                let is_playing = self.audio_engine.is_playing();

                let btn_text = if is_playing { "Pause" } else { "Play" };
                if ui.add_enabled(has_track, egui::Button::new(btn_text)).clicked() && has_track {
                    if let Err(e) = self.audio_engine.toggle_play_pause() {
                        eprintln!("Control Error: {}", e);
                    }
                }
                ui.add_space(10.0);

                if ui.add_enabled(has_track, egui::Button::new("Stop")).clicked() && has_track {
                    self.audio_engine.stop();
                    self.smoothed_spectrum.fill(0.0);
                    self.spectrogram_history.clear();
                    self.persistence_buffer.clear();
                    self.rms_smoothed = -60.0;
                    self.peak_hold = -60.0;
                }
            });
            ui.add_space(6.0);
        });

        // === CENTRAL VISUALIZATION ===
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.style_mut().visuals.widgets.inactive.bg_fill = styles::BG_PRIMARY;
            ui.style_mut().visuals.widgets.inactive.weak_bg_fill = styles::BG_PRIMARY;

            let data_lock = self.audio_data.lock().unwrap();
            if let Some(data) = &*data_lock {
                let is_playing = self.audio_engine.is_playing();
                let sample_rate = data.sample_rate;
                let playhead = self.audio_engine.position_samples(sample_rate);

                // --- Analysis driven by engine clock ---
                // Use larger window for phase meter visibility
                let phase_window = 2048;
                let (left, right) = Self::get_stereo_window(data, playhead, phase_window);

                // Use left channel for FFT (or mix: could average L+R)
                let mono_mix: Vec<f32> = left.iter().zip(right.iter())
                    .map(|(&l, &r)| (l + r) * 0.5)
                    .collect();

                let has_energy = self.smoothed_spectrum.iter().any(|&x| x > 0.0001);

                if mono_mix.len() >= FFT_SIZE && (is_playing || has_energy) {
                    // Center FFT window on playhead
                    let raw_spectrum = self.fft_processor.process(&mono_mix[..FFT_SIZE]);

                    if is_playing {
                        for i in 0..self.smoothed_spectrum.len() {
                            let target = raw_spectrum[i];
                            self.smoothed_spectrum[i] += (target - self.smoothed_spectrum[i]) * 0.3;
                        }
                    } else {
                        for val in self.smoothed_spectrum.iter_mut() {
                            *val *= 0.5;
                            if *val < 0.0001 { *val = 0.0; }
                        }
                    }

                    // Spectrogram history
                    if is_playing {
                        self.spectrogram_history.push(raw_spectrum.clone());
                        if self.spectrogram_history.len() > SPECTROGRAM_WIDTH {
                            self.spectrogram_history.remove(0);
                        }
                    }

                    // RMS in dBFS
                    let rms = (mono_mix.iter().map(|s| s * s).sum::<f32>() / mono_mix.len() as f32).sqrt();
                    let db = if rms > 1e-6 { 20.0 * rms.log10() } else { -60.0 };
                    self.rms_smoothed += (db - self.rms_smoothed) * 0.15;

                    if db > self.peak_hold {
                        self.peak_hold = db;
                        self.peak_decay_timer = 1.0;
                    } else if self.peak_decay_timer > 0.0 {
                        self.peak_decay_timer -= 1.0 / 60.0;
                    } else {
                        self.peak_hold += (db - self.peak_hold) * 0.05;
                    }

                    // Persistence oscilloscope
                    if is_playing {
                        let points: Vec<egui::Pos2> = mono_mix.iter()
                            .enumerate()
                            .step_by(4)
                            .map(|(i, &s)| egui::Pos2::new(i as f32, -s * 50.0))
                            .collect();
                        self.persistence_buffer.push(points);
                        if self.persistence_buffer.len() > PERSISTENCE_FRAMES {
                            self.persistence_buffer.remove(0);
                        }
                    }

                    // Music-reactive rotation: bass drives speed
                    let bass_end = (200.0 / (sample_rate as f32 / FFT_SIZE as f32)) as usize;
                    let bass_energy: f32 = raw_spectrum[..bass_end.min(raw_spectrum.len())]
                        .iter().sum::<f32>() / bass_end.max(1) as f32;
                    let bass_db = if bass_energy > 1e-6 { 20.0 * bass_energy.log10() + 60.0 } else { 0.0 };
                    let bass_norm = (bass_db / 60.0).clamp(0.0, 1.0);
                    self.radial_rotation += 0.002 + bass_norm * 0.008;
                } else if !is_playing && !has_energy {
                    self.smoothed_spectrum.fill(0.0);
                }

                renderer::draw_visualizer(
                    ui,
                    &self.overview_envelope,
                    &self.smoothed_spectrum,
                    playhead,
                    data.samples.len() / data.channels,
                    &self.spectrogram_history,
                    &self.persistence_buffer,
                    &left,
                    &right,
                    self.rms_smoothed,
                    self.peak_hold,
                    self.radial_rotation,
                    bass_norm_for_pulse(&self.smoothed_spectrum, sample_rate),
                    sample_rate,  // <-- NEW: pass sample rate for correct freq mapping
                );
            } else {
                ui.vertical_centered_justified(|ui| {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("Load an MP3 to begin")
                        .size(20.0)
                        .color(styles::TEXT_SECONDARY));
                });
            }
        });

        let needs_repaint = self.audio_engine.is_playing()
            || self.smoothed_spectrum.iter().any(|&x| x > 0.0001);
        if needs_repaint {
            ctx.request_repaint();
        }
    }
}

/// Compute bass energy normalized 0..1 for radial pulse effect
fn bass_norm_for_pulse(spectrum: &[f32], sample_rate: u32) -> f32 {
    let bass_end = (200.0 / (sample_rate as f32 / FFT_SIZE as f32)) as usize;
    let bass_energy: f32 = spectrum[..bass_end.min(spectrum.len())]
        .iter().sum::<f32>() / bass_end.max(1) as f32;
    if bass_energy > 1e-6 {
        ((20.0 * bass_energy.log10() + 60.0) / 60.0).clamp(0.0, 1.0)
    } else {
        0.0
    }
}