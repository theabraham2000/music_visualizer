use rodio::{Decoder, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub fn decode_mp3(path: PathBuf) -> Result<Vec<f32>, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let source = Decoder::new(reader).map_err(|e| e.to_string())?;
    
    // Convert all samples to f32
    let samples: Vec<f32> = source.convert_samples().collect();
    Ok(samples)
}