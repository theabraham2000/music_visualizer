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