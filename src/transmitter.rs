use std::f64::consts::PI;
use std::fs::File;

use hound;
use hound::SampleFormat;

use crate::{
    AUDIO_BITS_PER_SAMPLE, AUDIO_SAMPLE_RATE, BIT_TONE_FREQUENCY_NEXT, BIT_TONE_FREQUENCY_OFF,
    BIT_TONE_FREQUENCY_ON, MAX_MAGNITUDE, TONE_LENGTH_US, TRANSMISSION_END_FREQUENCY,
    TRANSMISSION_START_FREQUENCY,
};

struct WaveFile {
    writer: hound::WavWriter<std::io::BufWriter<File>>,
}

impl WaveFile {
    fn new(filename: &str) -> Result<Self, hound::Error> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: AUDIO_SAMPLE_RATE,
            bits_per_sample: AUDIO_BITS_PER_SAMPLE,
            sample_format: SampleFormat::Int,
        };

        let writer = hound::WavWriter::create(filename, spec)?;
        Ok(WaveFile { writer })
    }

    fn add_tone(&mut self, frequency: u32, duration: u32) -> Result<(), hound::Error> {
        let num_samples = (AUDIO_SAMPLE_RATE * duration) / 1_000_000;
        let period = AUDIO_SAMPLE_RATE as f64 / frequency as f64;

        for i in 0..num_samples {
            let sample = ((MAX_MAGNITUDE / 2.0) * (2.0 * PI * i as f64 / period).sin()) as i32;
            self.writer.write_sample(sample)?;
        }

        Ok(())
    }
}

fn transmit_byte(wav: &mut WaveFile, byte: u8) -> Result<(), hound::Error> {
    for i in (0..8).rev() {
        let is_bit_set = (byte & (1 << i)) != 0;
        let freq = if is_bit_set {
            BIT_TONE_FREQUENCY_ON
        } else {
            BIT_TONE_FREQUENCY_OFF
        };
        wav.add_tone(freq, TONE_LENGTH_US)?;
        wav.add_tone(BIT_TONE_FREQUENCY_NEXT, TONE_LENGTH_US)?;
    }

    Ok(())
}

pub fn generate_audio_data(filename: &str, data: &[u8]) -> Result<(), hound::Error> {
    let mut wav = WaveFile::new(filename)?;

    wav.add_tone(TRANSMISSION_START_FREQUENCY, TONE_LENGTH_US)?;
    wav.add_tone(BIT_TONE_FREQUENCY_NEXT, TONE_LENGTH_US)?;
    for &byte in data {
        let result = transmit_byte(&mut wav, byte);
        if let Err(error) = result {
            println!("Error: {}", error);
        }
    }
    wav.add_tone(TRANSMISSION_END_FREQUENCY, TONE_LENGTH_US)?;

    Ok(())
}
