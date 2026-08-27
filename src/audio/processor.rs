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