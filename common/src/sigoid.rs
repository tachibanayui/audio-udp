use std::f32::consts::PI;


pub struct SigoidWaveIter {
    sample_rate: u32,

    sample_clock: u32,
    c_freq: f32,
}

impl SigoidWaveIter {
    pub fn new(sample_rate: u32, c_freq: f32) -> Self {
        Self {
            sample_rate,
            sample_clock: 0,
            c_freq,
        }
    }
}

impl Iterator for SigoidWaveIter {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.sample_clock += 1;
        self.sample_clock %= self.sample_rate;

        let next =
            (self.sample_clock as f32 * self.c_freq * 2.0 * PI / self.sample_rate as f32).sin();
        Some(next)
    }
}
