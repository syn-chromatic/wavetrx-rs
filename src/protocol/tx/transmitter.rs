use std::f32::consts;
use std::fs::File;
use std::io::BufWriter;

use hound;
use hound::{SampleFormat, WavSpec, WavWriter};

use crate::audio::utils::get_bit_depth_magnitudes;
use crate::protocol::profile::ProtocolProfile;

pub struct ToneGenerator {
    writer: WavWriter<BufWriter<File>>,
    spec: WavSpec,
    p_magnitude: f32,
    n_magnitude: f32,
}

impl ToneGenerator {
    pub fn new(filename: &str, spec: WavSpec) -> Result<Self, hound::Error> {
        let writer: WavWriter<BufWriter<File>> = WavWriter::create(filename, spec)?;
        let (p_magnitude, n_magnitude): (f32, f32) = get_bit_depth_magnitudes(&spec);
        let tone_generator: ToneGenerator = ToneGenerator {
            writer,
            spec,
            p_magnitude,
            n_magnitude,
        };
        Ok(tone_generator)
    }

    pub fn append_tone(&mut self, frequency: f32, duration: usize) -> Result<(), hound::Error> {
        let sample_rate: usize = self.spec.sample_rate as usize;

        let sample_size: usize = (sample_rate * duration) / 1_000_000;
        let period: f32 = sample_rate as f32 / frequency;

        for idx in 0..sample_size {
            let sine_magnitude: f32 = self.get_sine_magnitude(idx, period);
            self.writer.write_sample(sine_magnitude as i32)?;
        }

        Ok(())
    }

    pub fn append_sine_faded_tone(
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

    pub fn append_linear_faded_tone(
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

pub struct Transmitter {
    profile: ProtocolProfile,
    sample_rate: usize,
    bit_depth: usize,
}

impl Transmitter {
    pub fn new(profile: ProtocolProfile, sample_rate: usize, bit_depth: usize) -> Self {
        Transmitter {
            profile,
            sample_rate,
            bit_depth,
        }
    }

    pub fn create_file(&self, filename: &str, data: &[u8]) -> Result<(), hound::Error> {
        let spec: WavSpec = self.get_wave_spec();
        let mut tone: ToneGenerator = ToneGenerator::new(filename, spec)?;
        let fade_ratio: f32 = 0.1;

        self.append_start(&mut tone, fade_ratio)?;
        self.append_next(&mut tone, fade_ratio)?;

        for &byte in data.iter() {
            self.append_byte(&mut tone, byte, fade_ratio)?;
        }

        self.append_end(&mut tone, fade_ratio)?;
        self.append_next(&mut tone, fade_ratio)?;
        Ok(())
    }
}

impl Transmitter {
    fn get_wave_spec(&self) -> WavSpec {
        let spec: WavSpec = WavSpec {
            channels: 1,
            sample_rate: self.sample_rate as u32,
            bits_per_sample: self.bit_depth as u16,
            sample_format: SampleFormat::Int,
        };
        spec
    }

    fn append_byte(
        &self,
        tone: &mut ToneGenerator,
        byte: u8,
        fade_ratio: f32,
    ) -> Result<(), hound::Error> {
        for i in (0..8).rev() {
            let bit: bool = (byte & (1 << i)) != 0;
            self.append_bit(tone, bit, fade_ratio)?;
            self.append_next(tone, fade_ratio)?;
        }
        Ok(())
    }

    fn append_start(&self, tone: &mut ToneGenerator, fade_ratio: f32) -> Result<(), hound::Error> {
        let tone_length: usize = self.profile.tone_length;
        let gap_length: usize = self.profile.gap_length;
        let frequency: f32 = self.profile.start;

        tone.append_sine_faded_tone(frequency, tone_length, fade_ratio)?;
        tone.append_tone(0.0, gap_length)?;
        Ok(())
    }

    fn append_end(&self, tone: &mut ToneGenerator, fade_ratio: f32) -> Result<(), hound::Error> {
        let tone_length: usize = self.profile.tone_length;
        let gap_length: usize = self.profile.gap_length;
        let frequency: f32 = self.profile.end;

        tone.append_sine_faded_tone(frequency, tone_length, fade_ratio)?;
        tone.append_tone(0.0, gap_length)?;
        Ok(())
    }

    fn append_next(&self, tone: &mut ToneGenerator, fade_ratio: f32) -> Result<(), hound::Error> {
        let tone_length: usize = self.profile.tone_length;
        let gap_length: usize = self.profile.gap_length;
        let frequency: f32 = self.profile.next;

        tone.append_sine_faded_tone(frequency, tone_length, fade_ratio)?;
        tone.append_tone(0.0, gap_length)?;
        Ok(())
    }

    fn append_bit(
        &self,
        tone: &mut ToneGenerator,
        bit: bool,
        fade_ratio: f32,
    ) -> Result<(), hound::Error> {
        let frequency: f32 = self.get_frequency_bit(bit);
        let tone_length: usize = self.profile.tone_length;
        let gap_length: usize = self.profile.gap_length;

        tone.append_sine_faded_tone(frequency, tone_length, fade_ratio)?;
        tone.append_tone(0.0, gap_length)?;
        Ok(())
    }

    fn get_frequency_bit(&self, bit: bool) -> f32 {
        if bit {
            return self.profile.high;
        }
        self.profile.low
    }
}
