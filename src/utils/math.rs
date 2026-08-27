pub fn normalize(value: f32, min: f32, max: f32) -> f32 {
    if max == min { return 0.0; }
    (value - min) / (max - min)
}