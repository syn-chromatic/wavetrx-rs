use std::f32::consts;

use hound;
use hound::WavSpec;

use crate::audio::utils::get_bit_depth_magnitudes;

pub struct ToneGenerator {
    samples: Vec<i32>,
    spec: WavSpec,
    p_magnitude: f32,
    n_magnitude: f32,
}

impl ToneGenerator {
    pub fn new(spec: WavSpec) -> Result<Self, Box<dyn std::error::Error>> {
        let samples: Vec<i32> = Vec::new();
        let (p_magnitude, n_magnitude): (f32, f32) = get_bit_depth_magnitudes(&spec);
        let tone_generator: ToneGenerator = ToneGenerator {
            samples,
            spec,
            p_magnitude,
            n_magnitude,
        };
        Ok(tone_generator)
    }

    pub fn samples(self) -> Vec<i32> {
        self.samples
    }

    pub fn append_tone(
        &mut self,
        frequency: f32,
        duration: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate: usize = self.spec.sample_rate as usize;

        let sample_size: usize = (sample_rate * duration) / 1_000_000;
        let period: f32 = sample_rate as f32 / frequency;

        for idx in 0..sample_size {
            let sine_magnitude: f32 = self.get_sine_magnitude(idx, period);
            self.samples.push(sine_magnitude as i32);
        }

        Ok(())
    }

    pub fn append_sine_faded_tone(
        &mut self,
        frequency: f32,
        duration: usize,
        fade_ratio: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate: usize = self.spec.sample_rate as usize;
        let sample_size: usize = ((sample_rate * duration) / 1_000_000) as usize;
        let period: f32 = sample_rate as f32 / frequency;
        let fade_size: usize = (sample_size as f32 * fade_ratio) as usize;

        for idx in 0..sample_size {
            let mut sine_magnitude: f32 = self.get_sine_magnitude(idx, period);
            let fade_coefficient: f32 = self.get_sine_fade_coeff(idx, sample_size, fade_size);
            sine_magnitude *= fade_coefficient;
            self.samples.push(sine_magnitude as i32);
        }

        Ok(())
    }

    pub fn append_linear_faded_tone(
        &mut self,
        frequency: f32,
        duration: usize,
        fade_ratio: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate: usize = self.spec.sample_rate as usize;
        let sample_size: usize = ((sample_rate * duration) / 1_000_000) as usize;
        let period: f32 = sample_rate as f32 / frequency;
        let fade_size: usize = (sample_size as f32 * fade_ratio) as usize;

        for idx in 0..sample_size {
            let mut sine_magnitude: f32 = self.get_sine_magnitude(idx, period);
            let fade_coefficient: f32 = self.get_linear_fade_coeff(idx, sample_size, fade_size);
            sine_magnitude *= fade_coefficient;
            self.samples.push(sine_magnitude as i32);
        }

        Ok(())
    }
}

impl ToneGenerator {
    fn get_sine_magnitude(&self, idx: usize, period: f32) -> f32 {
        let sine_norm: f32 = (2.0 * consts::PI * idx as f32 / period).sin();
        if sine_norm.is_sign_positive() {
            return sine_norm * self.p_magnitude;
        }
        sine_norm * self.n_magnitude.abs()
    }

    fn get_sine_fade_coeff(&self, idx: usize, sample_size: usize, fade_size: usize) -> f32 {
        let fade_coefficient: f32 = if idx < fade_size {
            0.5 * (1.0 - (consts::PI * idx as f32 / fade_size as f32).cos())
        } else if idx >= sample_size - fade_size {
            let relative_i: usize = idx - (sample_size - fade_size);
            0.5 * (1.0 + (consts::PI * relative_i as f32 / fade_size as f32).cos())
        } else {
            1.0
        };
        fade_coefficient
    }

    fn get_linear_fade_coeff(&self, idx: usize, sample_size: usize, fade_size: usize) -> f32 {
        let fade_coefficient: f32 = if idx < fade_size {
            idx as f32 / fade_size as f32
        } else if idx >= sample_size - fade_size {
            (sample_size - idx) as f32 / fade_size as f32
        } else {
            1.0
        };
        fade_coefficient
    }
}
