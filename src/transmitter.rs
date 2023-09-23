use std::f32::consts;
use std::fs::File;
use std::io::BufWriter;

use hound;
use hound::{SampleFormat, WavSpec, WavWriter};

use crate::{
    AUDIO_BPS, AUDIO_SR, BIT_FREQUENCY_NEXT, BIT_FREQUENCY_OFF, BIT_FREQUENCY_ON, TONE_GAP_US,
    TONE_LENGTH_US, TRANSMIT_END_FREQUENCY, TRANSMIT_START_FREQUENCY,
};

struct WaveFile {
    writer: WavWriter<BufWriter<File>>,
    spec: WavSpec,
    max_magnitude: f32,
}

impl WaveFile {
    fn new(filename: &str) -> Result<Self, hound::Error> {
        let spec: WavSpec = WavSpec {
            channels: 1,
            sample_rate: AUDIO_SR as u32,
            bits_per_sample: AUDIO_BPS as u16,
            sample_format: SampleFormat::Int,
        };

        let writer: WavWriter<BufWriter<File>> = WavWriter::create(filename, spec)?;
        let max_magnitude: f32 = ((2usize.pow(AUDIO_BPS as u32 - 1)) - 1) as f32;
        Ok(WaveFile {
            writer,
            spec,
            max_magnitude,
        })
    }

    fn add_tone(&mut self, frequency: f32, duration: usize) -> Result<(), hound::Error> {
        let sample_rate: usize = self.spec.sample_rate as usize;

        let sample_size: usize = (sample_rate * duration) / 1_000_000;
        let period: f32 = sample_rate as f32 / frequency;

        for idx in 0..sample_size {
            let sine_magnitude: f32 = self.get_sine_magnitude(idx, period);
            self.writer.write_sample(sine_magnitude as i32)?;
        }

        Ok(())
    }

    fn add_sine_faded_tone(
        &mut self,
        frequency: f32,
        duration: usize,
        fade_ratio: f32,
    ) -> Result<(), hound::Error> {
        let sample_rate: usize = self.spec.sample_rate as usize;
        let sample_size: usize = ((sample_rate * duration) / 1_000_000) as usize;
        let period: f32 = sample_rate as f32 / frequency;
        let fade_size: usize = (sample_size as f32 * fade_ratio) as usize;

        for idx in 0..sample_size {
            let mut sine_magnitude: f32 = self.get_sine_magnitude(idx, period);
            let fade_coefficient: f32 = self.get_sine_fade_coeff(idx, sample_size, fade_size);
            sine_magnitude *= fade_coefficient;
            self.writer.write_sample(sine_magnitude as i32)?;
        }

        Ok(())
    }

    fn add_linear_faded_tone(
        &mut self,
        frequency: f32,
        duration: usize,
        fade_ratio: f32,
    ) -> Result<(), hound::Error> {
        let sample_rate: usize = self.spec.sample_rate as usize;
        let sample_size: usize = ((sample_rate * duration) / 1_000_000) as usize;
        let period: f32 = sample_rate as f32 / frequency;
        let fade_size: usize = (sample_size as f32 * fade_ratio) as usize;

        for idx in 0..sample_size {
            let mut sine_magnitude: f32 = self.get_sine_magnitude(idx, period);
            let fade_coefficient: f32 = self.get_linear_fade_coeff(idx, sample_size, fade_size);
            sine_magnitude *= fade_coefficient;
            self.writer.write_sample(sine_magnitude as i32)?;
        }

        Ok(())
    }

    fn get_sine_magnitude(&self, idx: usize, period: f32) -> f32 {
        let max_magnitude: f32 = self.max_magnitude;
        let sine_norm: f32 = (2.0 * consts::PI * idx as f32 / period).sin();
        let sine_magnitude: f32 = sine_norm * max_magnitude;
        sine_magnitude
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

fn transmit_byte(wav: &mut WaveFile, byte: u8, fade_ratio: f32) -> Result<(), hound::Error> {
    for i in (0..8).rev() {
        let is_bit_set: bool = (byte & (1 << i)) != 0;
        let freq: f32 = if is_bit_set {
            BIT_FREQUENCY_ON
        } else {
            BIT_FREQUENCY_OFF
        };
        wav.add_sine_faded_tone(freq, TONE_LENGTH_US, fade_ratio)?;
        wav.add_tone(0.0, TONE_GAP_US)?;
        wav.add_sine_faded_tone(BIT_FREQUENCY_NEXT, TONE_LENGTH_US, fade_ratio)?;
        wav.add_tone(0.0, TONE_GAP_US)?;
    }

    Ok(())
}

pub fn generate_audio_data(filename: &str, data: &[u8]) -> Result<(), hound::Error> {
    let mut wav: WaveFile = WaveFile::new(filename)?;
    let fade_ratio: f32 = 0.1;

    wav.add_tone(0.0, TONE_GAP_US)?;
    wav.add_sine_faded_tone(TRANSMIT_START_FREQUENCY, TONE_LENGTH_US, fade_ratio)?;
    wav.add_tone(0.0, TONE_GAP_US)?;
    wav.add_sine_faded_tone(BIT_FREQUENCY_NEXT, TONE_LENGTH_US, fade_ratio)?;
    wav.add_tone(0.0, TONE_GAP_US)?;

    for &byte in data {
        let result = transmit_byte(&mut wav, byte, fade_ratio);
        if let Err(error) = result {
            println!("Error: {}", error);
        }
    }
    wav.add_sine_faded_tone(TRANSMIT_END_FREQUENCY, TONE_LENGTH_US, fade_ratio)?;
    wav.add_tone(0.0, TONE_GAP_US)?;
    wav.add_sine_faded_tone(BIT_FREQUENCY_NEXT, TONE_LENGTH_US, fade_ratio)?;
    wav.add_tone(0.0, TONE_GAP_US)?;
    Ok(())
}
