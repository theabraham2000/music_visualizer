use rodio::{Decoder, Sink, OutputStream}; // Removed Source
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub struct AudioEngine {
    _stream: Option<OutputStream>,
    sink: Option<Sink>,
    current_path: Option<PathBuf>,
    is_playing: bool,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            _stream: None,
            sink: None,
            current_path: None,
            is_playing: false,
        }
    }

    pub fn load(&mut self, path: PathBuf) -> Result<(), String> {
        self.stop();
        self.current_path = Some(path.clone());

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
        Ok(())
    }

    pub fn toggle_play_pause(&mut self) -> Result<(), String> {
        if let Some(ref path) = self.current_path {
            if self.is_playing {
                if let Some(sink) = &self.sink {
                    sink.pause();
                }
                self.is_playing = false;
            } else {
                if self.sink.is_none() {
                    return self.load(path.clone());
                }
                
                if let Some(sink) = &self.sink {
                    sink.play();
                }
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
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn has_track(&self) -> bool {
        self.current_path.is_some()
    }
}