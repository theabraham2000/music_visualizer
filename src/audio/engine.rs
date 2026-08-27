use rodio::{Decoder, Sink, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub struct AudioEngine {
    _stream: Option<OutputStream>,
    sink: Option<Sink>,
    is_playing: bool,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            _stream: None,
            sink: None,
            is_playing: false,
        }
    }

    pub fn load_and_play(&mut self, path: PathBuf) -> Result<(), String> {
        self.stop();

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

    pub fn toggle_play_pause(&mut self) {
        if let Some(sink) = &self.sink {
            if self.is_playing {
                sink.pause();
                self.is_playing = false;
            } else {
                sink.play();
                self.is_playing = true;
            }
        }
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
        self.is_playing && self.sink.is_some()
    }

    pub fn is_loaded(&self) -> bool {
        self.sink.is_some()
    }
}