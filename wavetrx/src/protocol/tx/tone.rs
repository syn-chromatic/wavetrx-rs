use std::f32::consts;
use std::mem;

use crate::audio::types::AudioSpec;

pub struct ToneGenerator {
    samples: Vec<f32>,
    spec: AudioSpec,
}

impl ToneGenerator {
    pub fn new(spec: &AudioSpec) -> Result<Self, Box<dyn std::error::Error>> {
        let samples: Vec<f32> = Vec::new();
        let spec: AudioSpec = *spec;

        Ok(ToneGenerator { samples, spec })
    }

    pub fn samples(self) -> Vec<f32> {
        self.samples
    }

    pub fn take_samples(&mut self) -> Vec<f32> {
        let samples_len: usize = self.samples.len();
        let samples: Vec<f32> = mem::replace(&mut self.samples, Vec::with_capacity(samples_len));
        samples
    }

    pub fn append_tone(
        &mut self,
        frequency: f32,
        duration: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate: usize = self.spec.sample_rate() as usize;

        let sample_size: usize = (sample_rate * duration) / 1_000_000;
        let period: f32 = sample_rate as f32 / frequency;

        for idx in 0..sample_size {
            let sine_norm: f32 = self.get_sine_norm(idx, period);
            self.samples.push(sine_norm);
        }

        Ok(())
    }

    pub fn append_sine_faded_tone(
        &mut self,
        frequency: f32,
        duration: usize,
        fade: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate: usize = self.spec.sample_rate() as usize;
        let sample_size: usize = ((sample_rate * duration) / 1_000_000) as usize;
        let period: f32 = sample_rate as f32 / frequency;
        let fade_size: usize = (sample_size as f32 * fade) as usize;

        for idx in 0..sample_size {
            let mut sine_norm: f32 = self.get_sine_norm(idx, period);
            sine_norm *= self.get_sine_fade_coeff(idx, sample_size, fade_size);
            self.samples.push(sine_norm);
        }

        Ok(())
    }

    pub fn append_linear_faded_tone(
        &mut self,
        frequency: f32,
        duration: usize,
        fade: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sample_rate: usize = self.spec.sample_rate() as usize;
        let sample_size: usize = ((sample_rate * duration) / 1_000_000) as usize;
        let period: f32 = sample_rate as f32 / frequency;
        let fade_size: usize = (sample_size as f32 * fade) as usize;

        for idx in 0..sample_size {
            let mut sine_norm: f32 = self.get_sine_norm(idx, period);
            sine_norm *= self.get_linear_fade_coeff(idx, sample_size, fade_size);
            self.samples.push(sine_norm);
        }

        Ok(())
    }
}

impl ToneGenerator {
    fn get_sine_norm(&self, idx: usize, period: f32) -> f32 {
        (2.0 * consts::PI * idx as f32 / period).sin()
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
