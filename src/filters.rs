use std::fs::File;
use std::io::BufReader;

use biquad::ToHertz;
use biquad::{Biquad, Coefficients, DirectForm1, Hertz, Type};
use hound::{WavReader, WavSpec};

use crate::utils::save_audio;

pub fn apply_highpass_filter(samples: &mut Vec<f32>, highpass_frequency: f32, spec: WavSpec) {
    let fs: Hertz<f32> = spec.sample_rate.hz();
    let f0: Hertz<f32> = highpass_frequency.hz();
    let filter: Type = Type::HighPass;
    let q_value: f32 = 0.707;

    let coefficients: Result<Coefficients<f32>, biquad::Errors> =
        Coefficients::<f32>::from_params(filter, fs, f0, q_value);

    if let Ok(coefficients) = coefficients {
        let mut filter: DirectForm1<f32> = DirectForm1::<f32>::new(coefficients);

        for sample in samples.iter_mut() {
            *sample = filter.run(*sample);
        }
    }
}

pub fn apply_lowpass_filter(samples: &mut Vec<f32>, lowpass_frequency: f32, spec: WavSpec) {
    let fs: Hertz<f32> = spec.sample_rate.hz();
    let f0: Hertz<f32> = lowpass_frequency.hz();
    let filter: Type = Type::LowPass;
    let q_value: f32 = 0.707;

    let coefficients: Result<Coefficients<f32>, biquad::Errors> =
        Coefficients::<f32>::from_params(filter, fs, f0, q_value);

    if let Ok(coefficients) = coefficients {
        let mut filter: DirectForm1<f32> = DirectForm1::<f32>::new(coefficients);

        for sample in samples.iter_mut() {
            *sample = filter.run(*sample);
        }
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

    apply_highpass_filter(&mut samples, highpass_frequency, spec);
    apply_lowpass_filter(&mut samples, lowpass_frequency, spec);

    save_audio("Test.wav", &samples, spec);
}
