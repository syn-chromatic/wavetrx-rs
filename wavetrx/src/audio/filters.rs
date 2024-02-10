use std::fs::File;
use std::io::BufReader;

use biquad::ToHertz;
use biquad::{Biquad, Coefficients, DirectForm1, Hertz, Type};
use hound::{WavReader, WavSpec};

use super::types::AudioSpec;
use super::utils::get_bit_depth_magnitudes;
use super::utils::save_audio;

pub struct FrequencyPass<'a> {
    samples: &'a mut [f32],
    spec: &'a AudioSpec,
}

impl<'a> FrequencyPass<'a> {
    pub fn new(samples: &'a mut [f32], spec: &'a AudioSpec) -> Self {
        FrequencyPass { samples, spec }
    }

    pub fn apply_highpass(&mut self, frequency: f32, q_value: f32) {
        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            self.get_coefficients(Type::HighPass, frequency, q_value);

        if let Ok(coefficients) = coefficients {
            self.apply_coefficients(coefficients);
        }
    }

    pub fn apply_lowpass(&mut self, frequency: f32, q_value: f32) {
        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            self.get_coefficients(Type::LowPass, frequency, q_value);

        if let Ok(coefficients) = coefficients {
            self.apply_coefficients(coefficients);
        }
    }

    pub fn apply_bandpass(&mut self, lower_frequency: f32, upper_frequency: f32, sharpness: f32) {
        let center_frequency: f32 = (lower_frequency * upper_frequency).sqrt();
        let mut q_value: f32 = center_frequency / (upper_frequency - lower_frequency);
        q_value *= sharpness;

        for _ in 0..4 {
            let coefficients: Result<Coefficients<f32>, biquad::Errors> =
                self.get_coefficients(Type::BandPass, center_frequency, q_value);

            if let Ok(coefficients) = coefficients {
                self.apply_coefficients(coefficients);
            }
        }

        // let coefficients: Result<Coefficients<f32>, biquad::Errors> =
        //     self.get_coefficients(Type::BandPass, center_frequency, q_value);

        // if let Ok(coefficients) = coefficients {
        //     self.apply_coefficients(coefficients);
        // }
    }
}

impl<'a> FrequencyPass<'a> {
    fn get_coefficients(
        &self,
        filter: Type,
        frequency: f32,
        q_value: f32,
    ) -> Result<Coefficients<f32>, biquad::Errors> {
        let fs: Hertz<f32> = self.spec.sample_rate().hz();
        let f0: Hertz<f32> = frequency.hz();

        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            Coefficients::<f32>::from_params(filter, fs, f0, q_value);
        coefficients
    }

    fn apply_coefficients(&mut self, coefficients: Coefficients<f32>) {
        let (p_magnitude, n_magnitude): (f32, f32) = get_bit_depth_magnitudes(self.spec);
        let mut filter: DirectForm1<f32> = DirectForm1::<f32>::new(coefficients);

        for sample in self.samples.iter_mut() {
            *sample = filter.run(*sample);
            if sample.is_sign_positive() && *sample > p_magnitude {
                *sample = p_magnitude;
            } else if sample.is_sign_negative() && *sample < n_magnitude {
                *sample = n_magnitude;
            }
        }
    }
}

#[test]
fn test_function() {
    let filename: &str = "sweep_h.wav";
    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let spec: AudioSpec = reader.spec().into();
    // let encoding = spec.encoding();

    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let mut samples: Vec<f32> = samples.iter().map(|&sample| sample as f32).collect();

    let highpass_frequency: f32 = 1000.0;
    let lowpass_frequency: f32 = 1000.0;

    let mut filters: FrequencyPass<'_> = FrequencyPass::new(&mut samples, &spec);

    // filters.apply_highpass(highpass_frequency, 1.0);
    // filters.apply_lowpass(lowpass_frequency, 0.707);
    filters.apply_bandpass(5000.0, 10_000.0, 1.0);

    save_audio("test_filters1.wav", &samples, &spec);
}