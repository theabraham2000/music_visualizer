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