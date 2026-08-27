use rodio::{Decoder, Sink, OutputStream};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub struct AudioEngine {
    _stream: Option<OutputStream>,      // Keep stream alive
    sink: Option<Sink>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            _stream: None,
            sink: None,
        }
    }

    pub fn play(&mut self, path: PathBuf) -> Result<(), String> {
        // Stop previous playback
        if let Some(sink) = &self.sink {
            sink.stop();
        }

        // Create output stream (must be kept alive)
        let (_stream, stream_handle) = OutputStream::try_default().map_err(|e| e.to_string())?;

        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader).map_err(|e| e.to_string())?;

        let sink = Sink::try_new(&stream_handle).map_err(|e| e.to_string())?;
        sink.append(source);

        self._stream = Some(_stream);
        self.sink = Some(sink);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = &self.sink {
            sink.stop();
        }
        self.sink = None;
        self._stream = None;
    }
}