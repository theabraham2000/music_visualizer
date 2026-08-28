use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::Instant;
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

    overview_envelope: Vec<(f32, f32)>,

    start_time: Instant,
    anim_time: f32,
    beat_flash: f32,
    prev_bass_norm: f32,
    live_blink_timer: f32,
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
            start_time: Instant::now(),
            anim_time: 0.0,
            beat_flash: 0.0,
            prev_bass_norm: 0.0,
            live_blink_timer: 0.0,
        }
    }

    fn load_track(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.load_track_from_path(path);
        }
    }

    fn load_track_from_path(&mut self, path: std::path::PathBuf) {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        match loader::decode_mp3(path.clone()) {
            Ok(data) => {
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
                self.beat_flash = 0.0;
                self.prev_bass_norm = 0.0;

                let dur = self.audio_data.lock().unwrap().as_ref().unwrap().duration_secs;
                if let Err(e) = self.audio_engine.load(path, dur) {
                    eprintln!("Audio Error: {}", e);
                }
            }
            Err(e) => eprintln!("Decode Error: {}", e),
        }
    }

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
            for frame in start_frame..end_frame {
                let s = data.samples[frame];
                left.push(s);
                right.push(s);
            }
        }
        (left, right)
    }

    /// Draw a styled button with subtle fill overlay on hover (no bright stroke outlines)
    fn draw_button(
        ui: &mut egui::Ui,
        label: &str,
        is_primary: bool,
        text_color: egui::Color32,
        enabled: bool,
    ) -> bool {
        let padding = egui::vec2(16.0, 6.0);
        let font = egui::FontId::proportional(11.0);
        let galley = ui.fonts(|f| f.layout_no_wrap(label.to_string(), font, text_color));
        let size = galley.size() + padding * 2.0;
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let is_hovered = response.hovered() && enabled;
            let rounding = egui::Rounding::same(6.0);

            let bg = if !enabled {
                styles::BG_PANEL
            } else if is_primary {
                styles::BTN_PRIMARY_FILL
            } else {
                styles::BTN_SECONDARY_FILL
            };

            // Base fill
            ui.painter().rect_filled(rect, rounding, bg);

            // Subtle hover overlay (not a bright stroke)
            if is_hovered {
                ui.painter().rect_filled(rect, rounding, styles::BTN_HOVER_OVERLAY);
            }

            // Subtle border using panel border color (not neon)
            ui.painter().rect_stroke(rect, rounding, egui::Stroke::new(1.0, styles::PANEL_BORDER));

            let actual_text_color = if !enabled {
                styles::TEXT_DIM
            } else if is_primary {
                styles::BTN_PRIMARY_TEXT
            } else {
                text_color
            };
            ui.painter().galley(rect.center(), galley, actual_text_color);
        }

        response.clicked() && enabled
    }
}

impl eframe::App for VisualizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.audio_engine.check_finished();

        self.anim_time = self.start_time.elapsed().as_secs_f32();
        self.live_blink_timer = self.anim_time;

        // Smooth beat flash decay
        self.beat_flash *= 0.88;
        if self.beat_flash < 0.005 { self.beat_flash = 0.0; }

        // Smooth peak hold decay
        if self.peak_decay_timer > 0.0 {
            self.peak_decay_timer -= 1.0 / 60.0;
        } else {
            // Continuous smooth decay toward current RMS
            self.peak_hold += (self.rms_smoothed - self.peak_hold) * 0.03;
        }

        // Drag-and-drop file loading
        let dropped_path = ctx.input(|i| {
            i.raw.dropped_files.first().and_then(|f| f.path.clone())
        });
        if let Some(path) = dropped_path {
            self.load_track_from_path(path);
        }

        // === TOP BAR ===
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none().fill(styles::BG_DEEP).inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("MinSu").color(styles::ACCENT_GOLD).size(14.0).strong());
                    ui.label(egui::RichText::new("Music Visualizer").color(styles::TEXT_PRIMARY).size(14.0).strong());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let is_playing = self.audio_engine.is_playing();
                        let blink = (self.live_blink_timer * 1.8).sin() * 0.5 + 0.5;
                        let live_dot_color = if is_playing {
                            styles::with_alpha(styles::LIVE_ON, (70.0 + blink * 185.0) as u8)
                        } else {
                            styles::LIVE_OFF
                        };
                        let live_text_color = if is_playing { styles::LIVE_ON } else { styles::TEXT_DIM };

                        let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 2.5, live_dot_color);

                        ui.label(egui::RichText::new("LIVE").color(live_text_color).strong().size(9.0));
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("v0.7.0").color(styles::TEXT_DIM).size(8.0));
                    });
                });

                ui.add_space(1.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.current_track).color(styles::TEXT_PRIMARY).size(11.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let pos = self.audio_engine.position_seconds();
                        let dur = self.audio_engine.duration_seconds();
                        let fmt_time = |t: f32| -> String {
                            let m = (t / 60.0) as u32;
                            let s = (t % 60.0) as u32;
                            format!("{:02}:{:02}", m, s)
                        };
                        ui.label(egui::RichText::new(format!("{} / {}", fmt_time(pos), fmt_time(dur)))
                            .color(styles::ACCENT_GOLD)
                            .monospace()
                            .size(11.0));
                    });
                });
            });

        // === BOTTOM CONTROLS — Centered hierarchy ===
        egui::TopBottomPanel::bottom("controls")
            .frame(egui::Frame::none().fill(styles::BG_DEEP).inner_margin(egui::Margin::symmetric(12.0, 7.0)))
            .show(ctx, |ui| {
                // Subtle top separator
                let sep = ui.available_rect_before_wrap();
                ui.painter().line_segment(
                    [sep.left_top(), sep.right_top()],
                    egui::Stroke::new(1.0, styles::PANEL_BORDER),
                );
                ui.add_space(2.0);

                ui.horizontal_centered(|ui| {
                    let has_track = self.audio_engine.has_track();
                    let is_playing = self.audio_engine.is_playing();

                    // Load — secondary action (subtle fill)
                    if Self::draw_button(ui, "Load", false, styles::TEXT_PRIMARY, true) {
                        self.load_track();
                    }
                    ui.add_space(8.0);

                    // Play/Pause — primary action (gold fill)
                    let btn_label = if is_playing { "Pause" } else { "Play" };
                    if Self::draw_button(ui, btn_label, true, styles::BTN_PRIMARY_TEXT, has_track) {
                        if let Err(e) = self.audio_engine.toggle_play_pause() {
                            eprintln!("Control Error: {}", e);
                        }
                    }
                    ui.add_space(8.0);

                    // Stop — secondary with crimson text
                    if Self::draw_button(ui, "Stop", false, styles::BTN_STOP_TEXT, has_track) {
                        self.audio_engine.stop();
                        self.smoothed_spectrum.fill(0.0);
                        self.spectrogram_history.clear();
                        self.persistence_buffer.clear();
                        self.rms_smoothed = -60.0;
                        self.peak_hold = -60.0;
                    }

                    // Status indicator
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let status_text = if is_playing { "Playing" }
                                          else if has_track { "Paused" }
                                          else { "Ready" };
                        let status_color = if is_playing { styles::ACCENT_GOLD }
                                           else if has_track { styles::METER_AMBER }
                                           else { styles::TEXT_DIM };

                        ui.label(egui::RichText::new(status_text).color(status_color).size(10.0));

                        if is_playing {
                            let pulse = (self.anim_time * 3.0).sin() * 0.5 + 0.5;
                            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot_rect.center(), 2.5, styles::with_alpha(styles::ACCENT_GOLD, (50.0 + pulse * 205.0) as u8));
                        }
                    });
                });
            });

        // Deferred file dialog flag
        let mut should_open_file = false;
        // Deferred seek fraction
        let mut pending_seek: Option<f32> = None;

        // === CENTRAL VISUALIZATION ===
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(styles::BG_DEEP))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let data_lock = self.audio_data.lock().unwrap();
                    if let Some(data) = &*data_lock {
                        let is_playing = self.audio_engine.is_playing();
                        let sample_rate = data.sample_rate;
                        let playhead = self.audio_engine.position_samples(sample_rate);

                        let phase_window = 2048;
                        let (left, right) = Self::get_stereo_window(data, playhead, phase_window);

                        let mono_mix: Vec<f32> = left.iter().zip(right.iter())
                            .map(|(&l, &r)| (l + r) * 0.5)
                            .collect();

                        let has_energy = self.smoothed_spectrum.iter().any(|&x| x > 0.0001);

                        if mono_mix.len() >= FFT_SIZE && (is_playing || has_energy) {
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

                            if is_playing {
                                self.spectrogram_history.push(raw_spectrum.clone());
                                if self.spectrogram_history.len() > SPECTROGRAM_WIDTH {
                                    self.spectrogram_history.remove(0);
                                }
                            }

                            let rms = (mono_mix.iter().map(|s| s * s).sum::<f32>() / mono_mix.len() as f32).sqrt();
                            let db = if rms > 1e-6 { 20.0 * rms.log10() } else { -60.0 };
                            self.rms_smoothed += (db - self.rms_smoothed) * 0.15;

                            if db > self.peak_hold {
                                self.peak_hold = db;
                                self.peak_decay_timer = 1.0;
                            }

                            if is_playing {
                                let points: Vec<egui::Pos2> = mono_mix.iter()
                                    .enumerate()
                                    .step_by(4)
                                    .map(|(i, &s)| egui::Pos2::new(i as f32 * 0.1, -s * 50.0))
                                    .collect();
                                self.persistence_buffer.push(points);
                                if self.persistence_buffer.len() > PERSISTENCE_FRAMES {
                                    self.persistence_buffer.remove(0);
                                }
                            }

                            let bass_end = (200.0 / (sample_rate as f32 / FFT_SIZE as f32)) as usize;
                            let bass_energy: f32 = raw_spectrum[..bass_end.min(raw_spectrum.len())]
                                .iter().sum::<f32>() / bass_end.max(1) as f32;
                            let bass_db = if bass_energy > 1e-6 { 20.0 * bass_energy.log10() + 60.0 } else { 0.0 };
                            let bass_norm = (bass_db / 60.0).clamp(0.0, 1.0);

                            if bass_norm > 0.55 && self.prev_bass_norm <= 0.55 {
                                self.beat_flash = 1.0;
                            }
                            self.prev_bass_norm = bass_norm;

                            self.radial_rotation += 0.002 + bass_norm * 0.01;
                        } else if !is_playing && !has_energy {
                            self.smoothed_spectrum.fill(0.0);
                        }

                        let interaction = renderer::draw_visualizer(
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
                            sample_rate,
                            self.anim_time,
                            self.beat_flash,
                        );

                        if let Some(frac) = interaction.seek_fraction {
                            pending_seek = Some(frac);
                        }
                    } else {
                        // Empty state — clickable drop zone
                        let available = ui.available_size();
                        let drop_w = available.x.min(400.0);
                        let drop_h = 240.0_f32;
                        let drop_size = egui::vec2(drop_w, drop_h);

                        let offset_x = (available.x - drop_w) / 2.0;
                        if offset_x > 0.0 {
                            ui.add_space(offset_x);
                        }

                        let (drop_rect, drop_response) = ui.allocate_exact_size(drop_size, egui::Sense::click());

                        if ui.is_rect_visible(drop_rect) {
                            let is_hovered = drop_response.hovered();
                            let rounding = egui::Rounding::same(14.0);

                            // Subtle fill on hover
                            if is_hovered {
                                ui.painter().rect_filled(drop_rect.shrink(2.0), rounding, styles::with_alpha(styles::ACCENT_BLUE, 5));
                            }
                            ui.painter().rect_stroke(drop_rect.shrink(2.0), rounding, egui::Stroke::new(1.0, styles::PANEL_BORDER));

                            let c = drop_rect.center();
                            ui.painter().text(c + egui::vec2(0.0, -24.0), egui::Align2::CENTER_CENTER, "+", egui::FontId::proportional(36.0), styles::with_alpha(styles::ACCENT_GOLD, 80));
                            ui.painter().text(c + egui::vec2(0.0, 14.0), egui::Align2::CENTER_CENTER, "Drop MP3 here or click to Load", egui::FontId::proportional(13.0), styles::TEXT_DIM);
                            ui.painter().text(c + egui::vec2(0.0, 34.0), egui::Align2::CENTER_CENTER, "Your music deserves to be seen", egui::FontId::proportional(9.0), styles::with_alpha(styles::TEXT_DIM, 60));
                        }

                        if drop_response.clicked() {
                            should_open_file = true;
                        }
                    }
                });
            });

        // Deferred actions after MutexGuard is dropped
        if should_open_file {
            self.load_track();
        }
        if let Some(frac) = pending_seek {
            let dur = self.audio_engine.duration_seconds();
            self.audio_engine.seek_to(frac * dur);
        }

        // Repaint logic
        let needs_repaint = self.audio_engine.is_playing()
            || self.smoothed_spectrum.iter().any(|&x| x > 0.0001)
            || self.beat_flash > 0.005;
        if needs_repaint {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}

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