use std::fs::File;
use std::io::BufReader;

use biquad::ToHertz;
use biquad::{Biquad, Coefficients, DirectForm1, Hertz, Type};
use hound::{WavReader, WavSpec};

use crate::utils::save_audio;

pub struct FrequencyFilters<'a> {
    samples: &'a mut [f32],
    spec: &'a WavSpec,
}

impl<'a> FrequencyFilters<'a> {
    pub fn new(samples: &'a mut [f32], spec: &'a WavSpec) -> Self {
        FrequencyFilters { samples, spec }
    }

    pub fn apply_highpass(&mut self, frequency: f32) {
        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            self.get_coefficients(Type::HighPass, frequency);

        if let Ok(coefficients) = coefficients {
            self.apply_coefficients(coefficients);
        }
    }

    pub fn apply_lowpass(&mut self, frequency: f32) {
        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            self.get_coefficients(Type::LowPass, frequency);

        if let Ok(coefficients) = coefficients {
            self.apply_coefficients(coefficients);
        }
    }
}

impl<'a> FrequencyFilters<'a> {
    fn get_coefficients(
        &self,
        filter: Type,
        frequency: f32,
    ) -> Result<Coefficients<f32>, biquad::Errors> {
        let fs: Hertz<f32> = self.spec.sample_rate.hz();
        let f0: Hertz<f32> = frequency.hz();
        let q_value: f32 = 0.707;

        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            Coefficients::<f32>::from_params(filter, fs, f0, q_value);
        coefficients
    }

    fn apply_coefficients(&mut self, coefficients: Coefficients<f32>) {
        let bitrate_magnitude: f32 = self.get_bitrate_magnitude();
        let mut filter: DirectForm1<f32> = DirectForm1::<f32>::new(coefficients);

        for sample in self.samples.iter_mut() {
            *sample = filter.run(*sample);
            if sample.is_sign_positive() && *sample > bitrate_magnitude {
                *sample = bitrate_magnitude;
            } else if sample.is_sign_negative() && *sample < -bitrate_magnitude {
                *sample = -bitrate_magnitude;
            }
        }
    }

    fn get_bitrate_magnitude(&self) -> f32 {
        let bitrate: u32 = self.spec.bits_per_sample as u32;
        let bitrate_magnitude: f32 = ((2i32.pow(bitrate - 1)) - 1) as f32;
        bitrate_magnitude
    }
}

#[test]
fn test_function() {
    let filename = "transmitted_recording.wav";
    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let spec: WavSpec = reader.spec();

    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let mut samples: Vec<f32> = samples.iter().map(|&sample| sample as f32).collect();

    let highpass_frequency: f32 = 1000.0;
    let lowpass_frequency: f32 = 1000.0;

    let mut filters: FrequencyFilters<'_> = FrequencyFilters::new(&mut samples, &spec);
    filters.apply_highpass(highpass_frequency);
    filters.apply_lowpass(lowpass_frequency);

    save_audio("Test.wav", &samples, &spec);
}
