# Codebase Overview

This document contains the project structure and the source code of the repository.

# 1. Project Structure

```text
music_visualizer
├── AGENTS.md
└── src
    ├── app.rs
    ├── main.rs
    ├── audio
    │   ├── engine.rs
    │   ├── loader.rs
    │   ├── mod.rs
    │   └── processor.rs
    ├── utils
    │   ├── math.rs
    │   └── mod.rs
    └── viz
        ├── mod.rs
        ├── renderer.rs
        └── styles.rs
```

# 2. Source Code

## `AGENTS.md`

```
# AGENTS.md

## Instruction to coding agents

- Think and write code like a senior software architect and programmer.
- Never add emojis in the codebase.
- Keep the codebase clean and readable.
- Use Rust Programming language only.
```

## `src/app.rs`

```rust
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
```

## `src/audio/engine.rs`

```rust
use rodio::{Decoder, Sink, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct AudioEngine {
    _stream: Option<OutputStream>,
    sink: Option<Sink>,
    current_path: Option<PathBuf>,
    is_playing: bool,
    started_at: Option<Instant>,
    paused_position_secs: f32,
    duration_secs: f32,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            _stream: None,
            sink: None,
            current_path: None,
            is_playing: false,
            started_at: None,
            paused_position_secs: 0.0,
            duration_secs: 0.0,
        }
    }

    pub fn load(&mut self, path: PathBuf, duration_secs: f32) -> Result<(), String> {
        self.stop();
        self.current_path = Some(path.clone());
        self.duration_secs = duration_secs;
        self.paused_position_secs = 0.0;

        let (_stream, stream_handle) = OutputStream::try_default().map_err(|e| e.to_string())?;
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader).map_err(|e| e.to_string())?;

        let sink = Sink::try_new(&stream_handle).map_err(|e| e.to_string())?;
        sink.append(source);
        sink.play();

        self._stream = Some(_stream);
        self.sink = Some(sink);
        self.is_playing = true;
        self.started_at = Some(Instant::now());
        Ok(())
    }

    pub fn toggle_play_pause(&mut self) -> Result<(), String> {
        if let Some(ref path) = self.current_path {
            if self.is_playing {
                if let Some(sink) = &self.sink {
                    sink.pause();
                }
                if let Some(started) = self.started_at {
                    self.paused_position_secs += started.elapsed().as_secs_f32();
                }
                self.started_at = None;
                self.is_playing = false;
            } else {
                if self.sink.is_none() {
                    return self.load(path.clone(), self.duration_secs);
                }
                if let Some(sink) = &self.sink {
                    sink.play();
                }
                self.started_at = Some(Instant::now());
                self.is_playing = true;
            }
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = &self.sink {
            sink.stop();
        }
        self.sink = None;
        self._stream = None;
        self.is_playing = false;
        self.started_at = None;
        self.paused_position_secs = 0.0;
    }

    /// Seek to a specific position in seconds.
    pub fn seek_to(&mut self, position_secs: f32) {
        let clamped = position_secs.clamp(0.0, self.duration_secs);
        if let Some(sink) = &self.sink {
            let _ = sink.try_seek(Duration::from_secs_f32(clamped));
        }
        // Reset timing reference so position_seconds() stays accurate
        self.paused_position_secs = clamped;
        if self.is_playing {
            self.started_at = Some(Instant::now());
        }
    }

    pub fn position_seconds(&self) -> f32 {
        let elapsed = match (self.is_playing, self.started_at) {
            (true, Some(started)) => self.paused_position_secs + started.elapsed().as_secs_f32(),
            _ => self.paused_position_secs,
        };
        elapsed.min(self.duration_secs)
    }

    pub fn position_samples(&self, sample_rate: u32) -> usize {
        (self.position_seconds() * sample_rate as f32) as usize
    }

    pub fn duration_seconds(&self) -> f32 {
        self.duration_secs
    }

    pub fn check_finished(&mut self) {
        if self.is_playing {
            if let Some(sink) = &self.sink {
                if sink.empty() {
                    if let Some(started) = self.started_at {
                        self.paused_position_secs += started.elapsed().as_secs_f32();
                    }
                    self.started_at = None;
                    self.is_playing = false;
                }
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn has_track(&self) -> bool {
        self.current_path.is_some()
    }
}
```

## `src/audio/loader.rs`

```rust
use rodio::{Decoder, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub struct AudioData {
    pub samples: Vec<f32>,
    pub channels: usize,
    pub sample_rate: u32,
    pub duration_secs: f32,
}

pub fn decode_mp3(path: PathBuf) -> Result<AudioData, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let source = Decoder::new(reader).map_err(|e| e.to_string())?;

    let channels = source.channels() as usize;
    let sample_rate = source.sample_rate();
    let samples: Vec<f32> = source.convert_samples().collect();
    let duration_secs = samples.len() as f32 / (sample_rate as f32 * channels as f32);

    Ok(AudioData {
        samples,
        channels,
        sample_rate,
        duration_secs,
    })
}
```

## `src/audio/mod.rs`

```rust
pub mod loader;
pub mod engine;
pub mod processor; // <-- Added this line
```

## `src/audio/processor.rs`

```rust
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;

pub struct FftProcessor {
    fft: Arc<dyn rustfft::Fft<f32>>,
    buffer: Vec<Complex<f32>>,
}

impl FftProcessor {
    pub fn new(size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(size);
        Self {
            fft,
            buffer: vec![Complex::new(0.0, 0.0); size],
        }
    }

    /// Returns magnitude spectrum (normalized) for the given samples
    pub fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        let n = self.buffer.len();
        // Copy samples into complex buffer with Hann window
        for i in 0..n {
            let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos());
            self.buffer[i] = Complex::new(samples[i] * window, 0.0);
        }

        self.fft.process(&mut self.buffer);

        // Compute magnitudes and normalize
        self.buffer.iter()
            .take(n / 2) // Only positive frequencies
            .map(|c| c.norm() / n as f32)
            .collect()
    }
}
```

## `src/main.rs`

```rust
mod app;
mod audio;
mod viz;
mod utils;

use eframe::NativeOptions;
use eframe::egui::ViewportBuilder;

fn main() -> Result<(), eframe::Error> {
    let viewport = ViewportBuilder::default()
        .with_inner_size([960.0, 640.0])
        .with_min_inner_size([700.0, 500.0])
        .with_title("MinSu Music Visualizer");

    let options = NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "MinSu Music Visualizer",
        options,
        Box::new(|cc| {
            Box::new(app::VisualizerApp::new(cc)) as Box<dyn eframe::App>
        }),
    )
}
```

## `src/utils/math.rs`

```rust
pub fn normalize(value: f32, min: f32, max: f32) -> f32 {
    if max == min { return 0.0; }
    (value - min) / (max - min)
}
```

## `src/utils/mod.rs`

```rust
#[allow(dead_code)]
pub mod math;
```

## `src/viz/mod.rs`

```rust
#[allow(dead_code)]
pub mod renderer;
#[allow(dead_code)]
pub mod styles;
```

## `src/viz/renderer.rs`

```rust
use eframe::egui::{self, Pos2, Rect, Stroke, Color32, Rounding, Sense};
use eframe::epaint::{Mesh, PathShape, PathStroke};
use super::styles;
use std::sync::LazyLock;

// Logarithmic frequency band edges (Hz) for 96 bars spanning 20Hz-20kHz
static LOG_BAND_EDGES: LazyLock<[f32; 97]> = LazyLock::new(|| {
    let min_hz = 20.0_f32;
    let max_hz = 20000.0_f32;
    let log_min = min_hz.ln();
    let log_max = max_hz.ln();
    let mut bands = [0.0_f32; 97];
    for i in 0..97 {
        let t = i as f32 / 96.0;
        bands[i] = (log_min + t * (log_max - log_min)).exp();
    }
    bands
});

const RMS_SEGMENTS: usize = 48;

/// Result of waveform interaction: optional seek fraction (0..1)
pub struct WaveformInteraction {
    pub seek_fraction: Option<f32>,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_visualizer(
    ui: &mut egui::Ui,
    overview_envelope: &[(f32, f32)],
    spectrum: &[f32],
    playhead: usize,
    total_frames: usize,
    spectrogram_history: &[Vec<f32>],
    persistence_buffer: &[Vec<Pos2>],
    left_channel: &[f32],
    right_channel: &[f32],
    rms_db: f32,
    peak_db: f32,
    radial_rotation: f32,
    bass_pulse: f32,
    sample_rate: u32,
    anim_time: f32,
    beat_flash: f32,
) -> WaveformInteraction {
    let full_width = ui.available_width();
    let available_h = ui.available_height().max(480.0);

    // Proportional layout budget
    let radial_h = (available_h * 0.40).clamp(170.0, 300.0);
    let meter_h = 62.0_f32;
    let spectro_h = (available_h * 0.20).clamp(90.0, 150.0);
    let wave_h = (available_h - radial_h - meter_h - spectro_h - 12.0).max(64.0);

    // Background atmosphere
    let bg_rect = Rect::from_min_size(ui.min_rect().left_top(), egui::vec2(full_width, available_h));
    draw_background(ui.painter_at(bg_rect), bg_rect, beat_flash);

    // Radial hero
    let (_, radial_rect) = ui.allocate_space(egui::vec2(full_width, radial_h));
    draw_radial_hero(ui.painter_at(radial_rect), radial_rect, spectrum, radial_rotation, bass_pulse, sample_rate, anim_time);

    ui.add_space(3.0);

    // Meters row
    let mut _phase_corr: f32 = 0.0;
    ui.horizontal(|ui| {
        let half_w = full_width / 2.0 - 2.0;
        let (_, rms_rect) = ui.allocate_space(egui::vec2(half_w, meter_h));
        draw_rms_meter(ui.painter_at(rms_rect), rms_rect, rms_db, peak_db);

        ui.add_space(4.0);

        let (_, phase_rect) = ui.allocate_space(egui::vec2(half_w, meter_h));
        _phase_corr = draw_phase_meter(ui.painter_at(phase_rect), phase_rect, left_channel, right_channel, persistence_buffer);
    });

    ui.add_space(3.0);

    // Spectrogram
    let (_, spec_rect) = ui.allocate_space(egui::vec2(full_width, spectro_h));
    draw_spectrogram(ui.painter_at(spec_rect), spec_rect, spectrogram_history);

    ui.add_space(3.0);

    // Waveform with interaction
    let (wave_rect, wave_response) = ui.allocate_exact_size(egui::vec2(full_width, wave_h), Sense::click_and_drag());
    let seek_fraction = draw_waveform_timeline(ui.painter_at(wave_rect), wave_rect, overview_envelope, playhead, total_frames, &wave_response);

    WaveformInteraction { seek_fraction }
}

// =============================================================================
// BACKGROUND ATMOSPHERE
// =============================================================================
fn draw_background(painter: egui::Painter, rect: Rect, beat_flash: f32) {
    painter.rect_filled(rect, Rounding::ZERO, styles::BG_DEEP);

    // Subtle dot grid
    let spacing = 30.0_f32;
    let cols = (rect.width() / spacing) as usize;
    let rows = (rect.height() / spacing) as usize;
    for col in 0..=cols {
        for row in 0..=rows {
            let x = rect.left() + col as f32 * spacing;
            let y = rect.top() + row as f32 * spacing;
            painter.circle_filled(Pos2::new(x, y), 0.7, styles::GRID_DOT);
        }
    }

    // Beat flash overlay
    if beat_flash > 0.01 {
        let alpha = (beat_flash * 8.0) as u8;
        painter.rect_filled(rect, Rounding::ZERO, Color32::from_rgba_premultiplied(245, 176, 65, alpha));
    }
}

// =============================================================================
// RADIAL HERO — Smooth arcs with scale rings
// =============================================================================
fn draw_radial_hero(
    painter: egui::Painter,
    rect: Rect,
    spectrum: &[f32],
    rotation: f32,
    bass_pulse: f32,
    sample_rate: u32,
    anim_time: f32,
) {
    // Panel background
    painter.rect_filled(rect, Rounding::same(12.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, styles::PANEL_BORDER));

    let center = rect.center();
    let base_radius = rect.width().min(rect.height()) * 0.35;
    let pulse = 1.0 + bass_pulse * 0.15;
    let radius = base_radius * pulse;
    let inner_r = radius * 0.45;
    let bar_count = 96;
    let angle_step = std::f32::consts::TAU / bar_count as f32;
    let nyquist = sample_rate as f32 / 2.0;

    let has_energy = spectrum.iter().any(|&x| x > 0.0001);
    let breathe = if !has_energy { 1.0 + (anim_time * 1.0).sin() * 0.02 } else { 1.0 };

    // Scale rings at -40dB, -20dB, 0dB (normalized: 0.33, 0.67, 1.0)
    let db_rings: [(f32, &str); 3] = [(0.33, "-40"), (0.67, "-20"), (1.0, "0")];
    for (frac, label) in &db_rings {
        let ring_r = inner_r + frac * (radius - inner_r);
        // Dashed circle via short arc segments
        let dash_count = 60;
        let dash_angle = std::f32::consts::TAU / dash_count as f32;
        for i in 0..dash_count {
            if i % 3 != 0 { continue; } // skip 2 of every 3 for dashed effect
            let a1 = i as f32 * dash_angle;
            let a2 = (i + 1) as f32 * dash_angle;
            let p1 = Pos2::new(center.x + a1.cos() * ring_r, center.y + a1.sin() * ring_r);
            let p2 = Pos2::new(center.x + a2.cos() * ring_r, center.y + a2.sin() * ring_r);
            painter.line_segment([p1, p2], Stroke::new(0.5, styles::with_alpha(styles::PANEL_BORDER, 60)));
        }
        // Label at top of ring
        let label_pos = Pos2::new(center.x, center.y - ring_r - 2.0);
        painter.text(label_pos, egui::Align2::CENTER_BOTTOM, *label, egui::FontId::proportional(6.0), styles::TEXT_DIM);
    }

    // Draw bars as smooth rounded pill shapes using PathShape arcs
    // Glow pass first, then sharp pass
    for pass in 0..2 {
        let is_glow = pass == 0;
        let bar_half_width = if is_glow { 3.5 } else { 2.0 };
        let alpha_mult: f32 = if is_glow { 0.2 } else { 1.0 };

        for i in 0..bar_count {
            let low_hz = LOG_BAND_EDGES[i];
            let high_hz = LOG_BAND_EDGES[i + 1];
            let bin_low = ((low_hz / nyquist) * spectrum.len() as f32) as usize;
            let bin_high = ((high_hz / nyquist) * spectrum.len() as f32) as usize;
            let bin_low = bin_low.min(spectrum.len().saturating_sub(1));
            let bin_high = bin_high.min(spectrum.len()).max(bin_low + 1);

            let band_mag: f32 = spectrum[bin_low..bin_high].iter().sum::<f32>() / (bin_high - bin_low) as f32;
            let db = if band_mag > 1e-6 { 20.0 * band_mag.log10() } else { -60.0 };
            let normalized = ((db + 60.0) / 60.0).clamp(0.0, 1.0) * breathe;

            if normalized < 0.01 && !is_glow { continue; }

            let angle = i as f32 * angle_step + rotation - std::f32::consts::FRAC_PI_2;
            let outer_r = inner_r + normalized * (radius - inner_r);

            let base_color = if i < bar_count / 3 {
                styles::FREQ_LOW
            } else if i < 2 * bar_count / 3 {
                styles::FREQ_MID
            } else {
                styles::FREQ_HIGH
            };

            let color = if is_glow {
                styles::with_alpha(base_color, (alpha_mult * 255.0) as u8)
            } else {
                styles::lerp_color(base_color, Color32::WHITE, normalized * 0.35)
            };

            // Draw as a thick line segment with rounded appearance
            // For anti-aliased look, we draw the bar as a thin filled quad (pill shape)
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            // Perpendicular direction for bar width
            let perp_x = -sin_a;
            let perp_y = cos_a;

            let sx = center.x + cos_a * inner_r;
            let sy = center.y + sin_a * inner_r;
            let ex = center.x + cos_a * outer_r;
            let ey = center.y + sin_a * outer_r;

            // Four corners of the bar quad
            let hw = bar_half_width;
            let p1 = Pos2::new(sx + perp_x * hw, sy + perp_y * hw);
            let p2 = Pos2::new(ex + perp_x * hw, ey + perp_y * hw);
            let p3 = Pos2::new(ex - perp_x * hw, ey - perp_y * hw);
            let p4 = Pos2::new(sx - perp_x * hw, sy - perp_y * hw);

            // Rounded tip: add arc points at the outer end
            let tip_segments = 4;
            let mut points: Vec<Pos2> = vec![p1];
            for t in 0..=tip_segments {
                let frac = t as f32 / tip_segments as f32;
                let tip_angle = angle - std::f32::consts::PI + frac * std::f32::consts::PI;
                let tx = ex + tip_angle.cos() * hw;
                let ty = ey + tip_angle.sin() * hw;
                points.push(Pos2::new(tx, ty));
            }
            points.push(p4);

            let shape = PathShape::convex_polygon(points, color, Stroke::NONE);
            painter.add(shape);
        }
    }

    // Inner ring — smooth circle with subtle pulse
    let inner_ring_r = inner_r - 2.0;
    let ring_pulse = 1.0 + bass_pulse * 0.05;
    painter.circle_stroke(center, inner_ring_r * ring_pulse, Stroke::new(1.0, styles::RADIAL_BASE));

    // Outer boundary ring
    painter.circle_stroke(center, radius + 4.0, Stroke::new(0.5, styles::with_alpha(styles::PANEL_BORDER, 40)));

    // Center readout with surrounding ring indicator
    let center_text = if has_energy {
        let avg: f32 = spectrum.iter().sum::<f32>() / spectrum.len().max(1) as f32;
        let db_val = if avg > 1e-6 { 20.0 * avg.log10() } else { -60.0 };
        format!("{:.0} dB", db_val)
    } else {
        "MinSu".to_string()
    };
    let text_color = if has_energy { styles::ACCENT_GOLD } else { styles::TEXT_DIM };

    // Subtle ring around center text that pulses with bass
    let text_ring_r = inner_r * 0.38;
    let text_ring_alpha = (40.0 + bass_pulse * 60.0) as u8;
    painter.circle_stroke(center, text_ring_r, Stroke::new(1.0, styles::with_alpha(styles::ACCENT_GOLD, text_ring_alpha)));

    painter.text(center, egui::Align2::CENTER_CENTER, center_text, egui::FontId::monospace(12.0), text_color);

    // Frequency band labels along outer circumference
    let band_labels: [(f32, f32, &str, Color32); 3] = [
        (0.0, 1.0 / 3.0, "BASS", styles::FREQ_LOW),
        (1.0 / 3.0, 2.0 / 3.0, "MID", styles::FREQ_MID),
        (2.0 / 3.0, 1.0, "HIGH", styles::FREQ_HIGH),
    ];
    for (start_frac, end_frac, label, color) in &band_labels {
        let mid_frac = (start_frac + end_frac) / 2.0;
        let angle = mid_frac * std::f32::consts::TAU + rotation - std::f32::consts::FRAC_PI_2;
        let label_r = radius + 16.0;
        let pos = Pos2::new(center.x + angle.cos() * label_r, center.y + angle.sin() * label_r);
        painter.text(pos, egui::Align2::CENTER_CENTER, *label, egui::FontId::proportional(7.0), *color);
    }

    // Anchor frequency labels
    let markers: [(f32, &str); 4] = [(20.0, "20"), (200.0, "200"), (2000.0, "2k"), (20000.0, "20k")];
    for (hz, label) in &markers {
        let frac = ((*hz).ln() - 20.0_f32.ln()) / (20000.0_f32.ln() - 20.0_f32.ln());
        let angle = frac * std::f32::consts::TAU + rotation - std::f32::consts::FRAC_PI_2;
        let label_r = radius + 28.0;
        let pos = Pos2::new(center.x + angle.cos() * label_r, center.y + angle.sin() * label_r);
        painter.text(pos, egui::Align2::CENTER_CENTER, *label, egui::FontId::proportional(6.0), styles::TEXT_DIM);
    }

    // Spark particles on strong bass
    if bass_pulse > 0.7 {
        let spark_count = 8;
        for i in 0..spark_count {
            let angle = (i as f32 / spark_count as f32) * std::f32::consts::TAU + anim_time * 1.2;
            let dist = radius + 6.0 + bass_pulse * 15.0;
            let pos = Pos2::new(center.x + angle.cos() * dist, center.y + angle.sin() * dist);
            let alpha = ((bass_pulse - 0.7) / 0.3 * 100.0) as u8;
            painter.circle_filled(pos, 1.2, styles::with_alpha(styles::ACCENT_GOLD, alpha));
        }
    }
}

// =============================================================================
// SPECTROGRAM — High dynamic range colormap with grid lines
// =============================================================================
fn draw_spectrogram(painter: egui::Painter, rect: Rect, history: &[Vec<f32>]) {
    painter.rect_filled(rect, Rounding::same(10.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, styles::PANEL_BORDER));

    if history.is_empty() {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "SPECTROGRAM", egui::FontId::proportional(9.0), styles::TEXT_DIM);
        return;
    }

    let cols = history.len();
    let rows = 128;
    let cell_w = rect.width() / cols as f32;
    let cell_h = rect.height() / rows as f32;

    let mut mesh = Mesh::default();
    for (col_idx, column) in history.iter().enumerate() {
        let x = rect.left() + col_idx as f32 * cell_w;
        for row in 0..rows {
            let bin_idx = ((row as f32 / rows as f32).powf(1.5) * column.len() as f32) as usize;
            let mag = column.get(bin_idx).copied().unwrap_or(0.0);
            let db = if mag > 1e-6 { 20.0 * mag.log10() + 60.0 } else { 0.0 };
            let t = (db / 60.0).clamp(0.0, 1.0);
            let color = styles::spectro_colormap(t);
            let y = rect.bottom() - row as f32 * cell_h;
            let cr = Rect::from_min_size(Pos2::new(x, y - cell_h), egui::vec2(cell_w + 0.5, cell_h + 0.5));
            mesh.colored_vertex(cr.left_top(), color);
            mesh.colored_vertex(cr.right_top(), color);
            mesh.colored_vertex(cr.left_bottom(), color);
            mesh.colored_vertex(cr.right_bottom(), color);
            let idx = mesh.vertices.len() as u32 - 4;
            mesh.add_triangle(idx, idx + 1, idx + 2);
            mesh.add_triangle(idx + 1, idx + 3, idx + 2);
        }
    }
    painter.add(mesh);

    // Horizontal grid lines at octave divisions
    let octaves: [(f32, &str); 4] = [
        (0.0, "20 kHz"),
        (0.33, "2 kHz"),
        (0.66, "200 Hz"),
        (0.92, "20 Hz"),
    ];
    for (frac, label) in &octaves {
        let y = rect.top() + frac * rect.height();
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5, styles::with_alpha(styles::PANEL_BORDER, 50)),
        );
        painter.text(
            Pos2::new(rect.left() + 3.0, y - 1.0),
            egui::Align2::LEFT_BOTTOM,
            *label,
            egui::FontId::proportional(6.0),
            styles::TEXT_DIM,
        );
    }

    // Panel label
    painter.text(
        rect.right_top() + egui::vec2(-8.0, 4.0),
        egui::Align2::RIGHT_TOP,
        "SPECTROGRAM",
        egui::FontId::proportional(8.0),
        styles::TEXT_DIM,
    );
}

// =============================================================================
// STEREO PHASE METER — Autoscaled goniometer with correlation readout
// =============================================================================
fn draw_phase_meter(
    painter: egui::Painter,
    rect: Rect,
    left: &[f32],
    right: &[f32],
    _persistence_buffer: &[Vec<Pos2>],
) -> f32 {
    painter.rect_filled(rect, Rounding::same(10.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, styles::PANEL_BORDER));

    let center = rect.center();
    // Use 78% of available space for the scope
    let size = (rect.width().min(rect.height()) / 2.0) * 0.78;

    // Circular boundary
    painter.circle_stroke(center, size, Stroke::new(0.5, styles::with_alpha(styles::PANEL_BORDER, 80)));

    // Crosshair axes
    painter.line_segment(
        [Pos2::new(center.x - size, center.y), Pos2::new(center.x + size, center.y)],
        Stroke::new(0.5, styles::with_alpha(styles::TEXT_DIM, 50)),
    );
    painter.line_segment(
        [Pos2::new(center.x, center.y - size), Pos2::new(center.x, center.y + size)],
        Stroke::new(0.5, styles::with_alpha(styles::TEXT_DIM, 50)),
    );

    // 45-degree diagonal guides for M/S alignment
    let d = size * 0.707;
    painter.line_segment(
        [Pos2::new(center.x - d, center.y - d), Pos2::new(center.x + d, center.y + d)],
        Stroke::new(0.3, styles::with_alpha(styles::PHASE_POSITIVE, 20)),
    );
    painter.line_segment(
        [Pos2::new(center.x - d, center.y + d), Pos2::new(center.x + d, center.y - d)],
        Stroke::new(0.3, styles::with_alpha(styles::PHASE_NEGATIVE, 20)),
    );

    let len = left.len().min(right.len());
    if len < 2 {
        painter.text(rect.left_top() + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP, "STEREO PHASE", egui::FontId::proportional(8.0), styles::TEXT_DIM);
        return 0.0;
    }

    // Autoscale: find peak amplitude to normalize display
    let mut peak_amp: f32 = 0.001;
    let max_samples = 512.min(len);
    let step = (len / max_samples).max(1);
    for i in (0..len).step_by(step) {
        peak_amp = peak_amp.max(left[i].abs()).max(right[i].abs());
    }
    let scale_factor = (0.8 / peak_amp).min(1.0); // autoscale to fill 80% of scope

    // Subsample for clean trace
    let mut points: Vec<Pos2> = Vec::with_capacity(max_samples);
    let mut correlation_sum = 0.0_f32;
    let mut energy_sum = 0.0_f32;

    for i in (0..len).step_by(step) {
        let lx = left[i] * scale_factor;
        let ry = right[i] * scale_factor;
        let x = center.x + lx * size;
        let y = center.y - ry * size;
        points.push(Pos2::new(
            x.clamp(rect.left() + 2.0, rect.right() - 2.0),
            y.clamp(rect.top() + 2.0, rect.bottom() - 2.0),
        ));
        correlation_sum += left[i] * right[i];
        energy_sum += left[i] * left[i] + right[i] * right[i];
    }

    let correlation = if energy_sum > 1e-6 {
        (correlation_sum / (energy_sum * 0.5)).clamp(-1.0, 1.0)
    } else {
        0.0
    };

    // Color trace by correlation value
    let trace_color = if correlation > 0.3 {
        styles::lerp_color(styles::ACCENT_BLUE, styles::PHASE_POSITIVE, (correlation - 0.3) / 0.7)
    } else if correlation < -0.3 {
        styles::lerp_color(styles::ACCENT_BLUE, styles::PHASE_NEGATIVE, (-correlation - 0.3) / 0.7)
    } else {
        styles::ACCENT_BLUE
    };

    // Draw trace
    if points.len() > 1 {
        for pts in points.windows(2) {
            painter.line_segment([pts[0], pts[1]], Stroke::new(1.0, styles::with_alpha(trace_color, 160)));
        }
    }

    // Correlation readout (top-right)
    let corr_color = if correlation > 0.3 { styles::PHASE_POSITIVE } else if correlation < -0.3 { styles::PHASE_NEGATIVE } else { styles::ACCENT_BLUE };
    painter.text(
        rect.right_top() + egui::vec2(-8.0, 16.0),
        egui::Align2::RIGHT_TOP,
        format!("{:+.2}", correlation),
        egui::FontId::monospace(9.0),
        corr_color,
    );

    // Panel label
    painter.text(rect.left_top() + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP, "STEREO PHASE", egui::FontId::proportional(8.0), styles::TEXT_DIM);

    correlation
}

// =============================================================================
// RMS LOUDNESS METER — Continuous gradient bar with dB markings
// =============================================================================
fn draw_rms_meter(painter: egui::Painter, rect: Rect, rms_db: f32, peak_db: f32) {
    painter.rect_filled(rect, Rounding::same(10.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, styles::PANEL_BORDER));

    let margin = 10.0_f32;
    let bar_left = rect.left() + margin;
    let bar_right = rect.right() - margin - 55.0;
    let bar_top = rect.top() + 20.0;
    let bar_bottom = rect.bottom() - 14.0;
    let bar_height = bar_bottom - bar_top;
    let bar_width = bar_right - bar_left;
    let min_db = -60.0_f32;
    let max_db = 0.0_f32;

    let rms_frac = ((rms_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
    let peak_frac = ((peak_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);

    // Track background
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(bar_left, bar_top), Pos2::new(bar_right, bar_bottom)),
        Rounding::same(3.0),
        Color32::from_gray(8),
    );

    // Continuous gradient fill using mesh
    let fill_w = rms_frac * bar_width;
    if fill_w > 1.0 {
        let grad_steps = 32;
        let mut mesh = Mesh::default();
        for i in 0..=grad_steps {
            let t = i as f32 / grad_steps as f32;
            let x = bar_left + t * fill_w;
            let color = styles::meter_gradient(t * rms_frac);
            mesh.colored_vertex(Pos2::new(x, bar_top), color);
            mesh.colored_vertex(Pos2::new(x, bar_bottom), color);
            if i > 0 {
                let idx = mesh.vertices.len() as u32 - 2;
                mesh.add_triangle(idx - 2, idx - 1, idx);
                mesh.add_triangle(idx - 1, idx + 1, idx);
            }
        }
        painter.add(mesh);

        // Thin divider overlays for segment feel
        let seg_count = RMS_SEGMENTS;
        let active_segs = (rms_frac * seg_count as f32) as usize;
        for i in 0..active_segs {
            let x = bar_left + (i + 1) as f32 / seg_count as f32 * fill_w;
            if x < bar_left + fill_w {
                painter.line_segment(
                    [Pos2::new(x, bar_top), Pos2::new(x, bar_bottom)],
                    Stroke::new(0.5, styles::with_alpha(styles::BG_DEEP, 120)),
                );
            }
        }
    }

    // Peak hold indicator with smooth visual
    let peak_x = bar_left + peak_frac * bar_width;
    painter.line_segment(
        [Pos2::new(peak_x, bar_top - 1.0), Pos2::new(peak_x, bar_bottom + 1.0)],
        Stroke::new(1.5, styles::PEAK_HOLD),
    );

    // dB scale markings below the bar
    let db_marks: [f32; 7] = [-60.0, -48.0, -36.0, -24.0, -12.0, -6.0, 0.0];
    for &db_val in &db_marks {
        let frac = ((db_val - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
        let x = bar_left + frac * bar_width;
        painter.line_segment(
            [Pos2::new(x, bar_bottom + 1.0), Pos2::new(x, bar_bottom + 3.5)],
            Stroke::new(0.5, styles::TEXT_DIM),
        );
        let label = if db_val == 0.0 { "0".to_string() } else { format!("{}", db_val as i32) };
        painter.text(
            Pos2::new(x, bar_bottom + 5.0),
            egui::Align2::CENTER_TOP,
            label,
            egui::FontId::proportional(5.5),
            styles::TEXT_DIM,
        );
    }

    // Large dB readout
    painter.text(
        Pos2::new(rect.right() - margin, bar_top + bar_height / 2.0),
        egui::Align2::RIGHT_CENTER,
        format!("{:.1}", rms_db),
        egui::FontId::monospace(13.0),
        styles::ACCENT_GOLD,
    );
    painter.text(
        Pos2::new(rect.right() - margin, bar_top + bar_height / 2.0 + 10.0),
        egui::Align2::RIGHT_CENTER,
        "dB",
        egui::FontId::proportional(7.0),
        styles::TEXT_DIM,
    );

    // Panel label
    painter.text(rect.left_top() + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP, "RMS LOUDNESS", egui::FontId::proportional(8.0), styles::TEXT_DIM);
}

// =============================================================================
// WAVEFORM TIMELINE — Interactive dual-tone envelope with glowing playhead
// =============================================================================
fn draw_waveform_timeline(
    painter: egui::Painter,
    rect: Rect,
    envelope: &[(f32, f32)],
    playhead: usize,
    total_frames: usize,
    response: &egui::Response,
) -> Option<f32> {
    painter.rect_filled(rect, Rounding::same(10.0), styles::BG_PANEL);
    painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0, styles::PANEL_BORDER));

    if envelope.is_empty() || total_frames == 0 {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "WAVEFORM", egui::FontId::proportional(9.0), styles::TEXT_DIM);
        return None;
    }

    let mid_y = rect.center().y;
    let margin_x = 8.0_f32;
    let width = rect.width() - margin_x * 2.0;
    let start_x = rect.left() + margin_x;
    let half_h = rect.height() / 2.0 - 8.0;
    let progress = (playhead as f32 / total_frames as f32).clamp(0.0, 1.0);
    let head_x = start_x + progress * width;
    let step = width / envelope.len() as f32;

    // Played region fill (brighter)
    let mut played_mesh = Mesh::default();
    let mut unplayed_mesh = Mesh::default();

    for (i, &(min_s, max_s)) in envelope.iter().enumerate() {
        let x = start_x + i as f32 * step;
        let y_min = mid_y - min_s * half_h;
        let y_max = mid_y - max_s * half_h;
        let is_played = x <= head_x;
        let fill_color = if is_played { styles::WAVE_PLAYED } else { styles::WAVE_UNPLAYED };
        let target_mesh = if is_played { &mut played_mesh } else { &mut unplayed_mesh };

        target_mesh.colored_vertex(Pos2::new(x, y_max), fill_color);
        target_mesh.colored_vertex(Pos2::new(x, y_min), fill_color);

        if i > 0 {
            let prev_x = start_x + (i - 1) as f32 * step;
            if (prev_x <= head_x) == is_played {
                let idx = target_mesh.vertices.len() as u32 - 2;
                target_mesh.add_triangle(idx - 2, idx - 1, idx);
                target_mesh.add_triangle(idx - 1, idx + 1, idx);
            }
        }
    }
    painter.add(unplayed_mesh);
    painter.add(played_mesh);

    // Envelope outline stroke
    for (i, &(min_s, max_s)) in envelope.iter().enumerate() {
        if i == 0 { continue; }
        let x = start_x + i as f32 * step;
        let prev_x = start_x + (i - 1) as f32 * step;
        let played = x <= head_x;
        let color = if played {
            styles::with_alpha(styles::ACCENT_BLUE, 110)
        } else {
            styles::with_alpha(styles::TEXT_DIM, 35)
        };
        let (prev_min, prev_max) = envelope[i - 1];
        painter.line_segment(
            [Pos2::new(prev_x, mid_y - prev_max * half_h), Pos2::new(x, mid_y - max_s * half_h)],
            Stroke::new(0.7, color),
        );
        painter.line_segment(
            [Pos2::new(prev_x, mid_y - prev_min * half_h), Pos2::new(x, mid_y - min_s * half_h)],
            Stroke::new(0.7, color),
        );
    }

    // Full-height glowing playhead line
    // Glow pass
    painter.line_segment(
        [Pos2::new(head_x, rect.top() + 2.0), Pos2::new(head_x, rect.bottom() - 2.0)],
        Stroke::new(5.0, styles::with_alpha(styles::ACCENT_GOLD, 20)),
    );
    // Sharp line
    painter.line_segment(
        [Pos2::new(head_x, rect.top() + 2.0), Pos2::new(head_x, rect.bottom() - 2.0)],
        Stroke::new(1.5, styles::PLAYHEAD_COLOR),
    );

    // Rounded top marker (small circle at top of playhead)
    painter.circle_filled(Pos2::new(head_x, rect.top() + 4.0), 3.0, styles::ACCENT_GOLD);

    // Hover timestamp tooltip
    if response.hovered() {
        if let Some(pointer_pos) = response.hover_pos() {
            let hover_frac = ((pointer_pos.x - start_x) / width).clamp(0.0, 1.0);
            let hover_secs = hover_frac * (total_frames as f32 / 44100.0); // approximate
            let m = (hover_secs / 60.0) as u32;
            let s = (hover_secs % 60.0) as u32;
            let tooltip_text = format!("{:02}:{:02}", m, s);
            let tooltip_pos = Pos2::new(pointer_pos.x, rect.top() - 2.0);
            painter.text(tooltip_pos, egui::Align2::CENTER_BOTTOM, tooltip_text, egui::FontId::monospace(9.0), styles::ACCENT_GOLD);
        }
    }

    // Panel label
    painter.text(rect.left_top() + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP, "WAVEFORM", egui::FontId::proportional(8.0), styles::TEXT_DIM);

    // Handle click/drag seeking
    if response.clicked() || response.dragged() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let seek_frac = ((pointer_pos.x - start_x) / width).clamp(0.0, 1.0);
            return Some(seek_frac);
        }
    }

    None
}
```

## `src/viz/styles.rs`

```rust
#[allow(dead_code)]
use eframe::egui::Color32;

// -- Deep Backgrounds ------------------------------------------------
pub const BG_DEEP: Color32 = Color32::from_rgb(13, 14, 21);       // #0D0E15
pub const BG_PANEL: Color32 = Color32::from_rgb(20, 22, 34);      // #141622
pub const BG_PANEL_HI: Color32 = Color32::from_rgb(30, 33, 50);   // hover state
pub const PANEL_BORDER: Color32 = Color32::from_rgb(42, 45, 66);  // #2A2D42

// Legacy aliases
pub const BG_PRIMARY: Color32 = BG_DEEP;
pub const PANEL_BG: Color32 = BG_PANEL;

// -- Accent Primary: Warm Gold / Amber --------------------------------
pub const ACCENT_GOLD: Color32 = Color32::from_rgb(245, 176, 65); // #F5B041

// -- Accent Dynamic: Soft Blue / Neon Violet --------------------------
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(112, 161, 255); // #70A1FF
pub const ACCENT_VIOLET: Color32 = Color32::from_rgb(165, 94, 234); // #A55EEA

// -- Meter Spectrum Gradient Stops ------------------------------------
pub const METER_GREEN: Color32 = Color32::from_rgb(46, 213, 115);  // #2ED573
pub const METER_AMBER: Color32 = Color32::from_rgb(255, 165, 2);   // #FFA502
pub const METER_CRIMSON: Color32 = Color32::from_rgb(255, 71, 87); // #FF4757

// -- Frequency Band Colors --------------------------------------------
pub const FREQ_LOW: Color32 = Color32::from_rgb(255, 71, 87);     // crimson for bass
pub const FREQ_MID: Color32 = Color32::from_rgb(245, 176, 65);    // gold for mids
pub const FREQ_HIGH: Color32 = Color32::from_rgb(165, 94, 234);   // violet for highs

// -- Text -------------------------------------------------------------
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(225, 228, 240);
pub const TEXT_DIM: Color32 = Color32::from_rgb(85, 90, 115);
pub const TEXT_SECONDARY: Color32 = TEXT_DIM;
pub const TEXT_GLOW: Color32 = ACCENT_GOLD;

// -- Waveform ---------------------------------------------------------
pub const WAVE_PLAYED: Color32 = Color32::from_rgba_premultiplied(112, 161, 255, 90);
pub const WAVE_UNPLAYED: Color32 = Color32::from_rgba_premultiplied(55, 58, 80, 45);
pub const WAVE_COLOR: Color32 = ACCENT_BLUE;
pub const WAVE_FILL: Color32 = WAVE_PLAYED;
pub const PLAYHEAD_COLOR: Color32 = ACCENT_GOLD;

// -- Spectrogram Colormap Stops (Navy -> Violet -> Magenta -> Amber -> White)
pub const SPEC_COLD: Color32 = Color32::from_rgb(10, 12, 30);
pub const SPEC_COOL: Color32 = Color32::from_rgb(80, 40, 140);
pub const SPEC_WARM: Color32 = Color32::from_rgb(200, 50, 120);
pub const SPEC_HOT: Color32 = Color32::from_rgb(255, 165, 2);
pub const SPEC_PEAK: Color32 = Color32::from_rgb(255, 245, 230);

// -- Radial & Meters --------------------------------------------------
pub const RADIAL_BASE: Color32 = Color32::from_rgba_premultiplied(112, 161, 255, 50);
pub const RMS_BAR: Color32 = ACCENT_GOLD;
pub const PEAK_HOLD: Color32 = Color32::from_rgb(255, 255, 255);
pub const PHASE_POSITIVE: Color32 = Color32::from_rgb(46, 213, 115);
pub const PHASE_NEGATIVE: Color32 = Color32::from_rgb(255, 71, 87);
pub const PERSISTENCE_FADE: Color32 = Color32::from_rgba_premultiplied(20, 22, 34, 40);

// -- Glow Helpers -----------------------------------------------------
pub const GLOW_CYAN: Color32 = Color32::from_rgba_premultiplied(112, 161, 255, 25);
pub const GLOW_MAGENTA: Color32 = Color32::from_rgba_premultiplied(255, 71, 87, 25);
pub const GLOW_VIOLET: Color32 = Color32::from_rgba_premultiplied(165, 94, 234, 25);

// -- Grid & Atmosphere ------------------------------------------------
pub const GRID_DOT: Color32 = Color32::from_rgba_premultiplied(112, 161, 255, 6);
pub const BEAT_FLASH: Color32 = Color32::from_rgba_premultiplied(245, 176, 65, 5);
pub const VIGNETTE_EDGE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 45);

// -- Buttons ----------------------------------------------------------
pub const BTN_PRIMARY_FILL: Color32 = ACCENT_GOLD;
pub const BTN_PRIMARY_TEXT: Color32 = Color32::from_rgb(13, 14, 21);
pub const BTN_SECONDARY_FILL: Color32 = BG_PANEL_HI;
pub const BTN_SECONDARY_TEXT: Color32 = TEXT_PRIMARY;
pub const BTN_HOVER_OVERLAY: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 12);
pub const BTN_STOP_TEXT: Color32 = METER_CRIMSON;

// -- Live Badge -------------------------------------------------------
pub const LIVE_ON: Color32 = Color32::from_rgb(255, 71, 87);
pub const LIVE_OFF: Color32 = TEXT_DIM;

// -- Backward-compat neon aliases -------------------------------------
pub const NEON_CYAN: Color32 = ACCENT_BLUE;
pub const NEON_MAGENTA: Color32 = METER_CRIMSON;
pub const NEON_VIOLET: Color32 = ACCENT_VIOLET;
pub const NEON_LIME: Color32 = METER_GREEN;
pub const NEON_AMBER: Color32 = METER_AMBER;
pub const BTN_OUTLINE_BORDER: Color32 = PANEL_BORDER;
pub const BTN_STOP_BORDER: Color32 = PANEL_BORDER;
pub const BTN_HOVER_GLOW: Color32 = BTN_HOVER_OVERLAY;

// -- Helper: color with custom alpha ----------------------------------
pub fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * a as f32 / 255.0) as u8,
        (c.g() as f32 * a as f32 / 255.0) as u8,
        (c.b() as f32 * a as f32 / 255.0) as u8,
        a,
    )
}

// -- Helper: lerp two colors ------------------------------------------
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

// -- Helper: meter spectrum gradient (green -> amber -> crimson) ------
pub fn meter_gradient(frac: f32) -> Color32 {
    let f = frac.clamp(0.0, 1.0);
    if f < 0.55 {
        lerp_color(METER_GREEN, METER_AMBER, f / 0.55)
    } else {
        lerp_color(METER_AMBER, METER_CRIMSON, (f - 0.55) / 0.45)
    }
}

// -- Helper: spectrogram colormap (navy -> violet -> magenta -> amber -> white)
pub fn spectro_colormap(t: f32) -> Color32 {
    let f = t.clamp(0.0, 1.0);
    if f < 0.2 {
        lerp_color(SPEC_COLD, SPEC_COOL, f / 0.2)
    } else if f < 0.45 {
        lerp_color(SPEC_COOL, SPEC_WARM, (f - 0.2) / 0.25)
    } else if f < 0.75 {
        lerp_color(SPEC_WARM, SPEC_HOT, (f - 0.45) / 0.3)
    } else {
        lerp_color(SPEC_HOT, SPEC_PEAK, (f - 0.75) / 0.25)
    }
}
```
