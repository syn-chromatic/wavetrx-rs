use std::fs::File;
use std::io::BufReader;

use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use rustfft::FftPlanner;

use hound::{WavReader, WavSpec};

use crate::audio::types::SampleEncoding;
use crate::audio::types::SampleSpec;
use crate::audio::utils::save_audio;

use crate::protocol::rx::spectrum::FourierMagnitude;
use crate::{
    BIT_FREQUENCY_NEXT, BIT_FREQUENCY_OFF, BIT_FREQUENCY_ON, TONE_LENGTH_US,
    TRANSMIT_END_FREQUENCY, TRANSMIT_START_FREQUENCY,
};

fn print_samples(samples: &[f32]) {
    let mut trunc_samples: Vec<f32> = samples.to_vec();
    trunc_samples.truncate(50);
    println!("Samples: {} | {:?}\n\n", trunc_samples.len(), trunc_samples);
}

fn attenuate_samples(samples: &mut Vec<f32>) {}

fn to_complex_samples(samples: &[f32]) -> Vec<Complex<f32>> {
    let mut complex_samples: Vec<Complex<f32>> = Vec::with_capacity(samples.len());
    for sample in samples.iter() {
        let complex = Complex::from_polar(*sample, 0.0);
        complex_samples.push(complex);
    }
    complex_samples
}

fn get_scratch_space(sample_size: usize) -> Vec<Complex<f32>> {
    let mut complex_samples: Vec<Complex<f32>> = Vec::with_capacity(sample_size);
    for _ in 0..sample_size {
        let complex: Complex<f32> = Complex::from(0.0);
        complex_samples.push(complex);
    }
    complex_samples
}

fn silence_bin(complex_samples: &mut Vec<Complex<f32>>, bin: usize, sample_size: usize) {
    if bin > 0 && bin != sample_size {
        complex_samples[bin].re = 0.0;
        complex_samples[bin].im = 0.0;
        complex_samples[sample_size - bin].re = 0.0;
        complex_samples[sample_size - bin].im = 0.0;
    }
}

fn maximize_bin(
    complex_samples: &mut Vec<Complex<f32>>,
    bin: usize,
    sample_size: usize,
    max_magnitude: f32,
) {
    // if bin > 0 && bin != sample_size {
    let complex_bin: Complex<f32> = complex_samples[bin];
    let complex_mirror: Complex<f32> = complex_samples[sample_size - bin];

    let normalization_factor: f32 = 2.0 / sample_size as f32;
    let bin_scale: f32 = complex_bin.norm() / normalization_factor;
    let mirror_scale: f32 = complex_mirror.norm() / normalization_factor;

    let bin_scale = bin_scale * 0.8;
    let mirror_scale = mirror_scale * 0.8;

    complex_samples[bin] = complex_bin.scale(max_magnitude - bin_scale);
    let max_magnitude = max_magnitude * 2.0;
    complex_samples[sample_size - bin] = complex_mirror.scale(max_magnitude - mirror_scale);

    println!(
        "Bin Norm: {} | Bin Sq: {} | Bin Sq2: {} | Scale: {}",
        complex_bin.norm(),
        complex_bin.norm_sqr(),
        complex_bin.norm_sqr().sqrt(),
        bin_scale
    );

    // }
}

fn get_bin(target_frequency: f32, sample_size: usize, sample_rate: usize) -> usize {
    let bin: usize = (target_frequency * (sample_size as f32) / sample_rate as f32) as usize;
    bin
}

#[test]
fn test_func() {
    let filename = "test5.wav";

    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let spec: SampleSpec = reader.spec().into();
    let audio_bps: usize = spec.bits_per_sample() as usize;
    let max_magnitude: f32 = ((2i32.pow(audio_bps as u32 - 1)) - 1) as f32;
    let sample_rate = spec.sample_rate() as usize;
    println!("Max Magnitude: {}", max_magnitude);

    let sample_size: usize = (sample_rate * TONE_LENGTH_US) as usize / 1_000_000;
    println!("Sample Size: {}", sample_size);

    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let mut samples: Vec<f32> = samples.iter().map(|&sample| sample as f32 - 1.0).collect();
    println!("Samples: {}", samples.len());
    let fft_magnitude = FourierMagnitude::new(sample_size, &spec);

    print_samples(&samples);

    let mut planner: FftPlanner<f32> = FftPlanner::new();
    let fft_forward = planner.plan_fft_forward(sample_size);
    let fft_inverse = planner.plan_fft_inverse(sample_size);
    let mut scratch: Vec<Complex<f32>> = get_scratch_space(sample_size);

    for i in (0..samples.len() - sample_size + 1).step_by(sample_size) {
        let samples_chunk: &[f32] = &samples[i..i + sample_size];
        let samples_chunk: Vec<f32> = samples_chunk.iter().map(|s| s / max_magnitude).collect();
        let mut complex_samples: Vec<Complex<f32>> = to_complex_samples(&samples_chunk);

        fft_forward.process(&mut complex_samples);

        let frequencies: Vec<f32> = vec![
            BIT_FREQUENCY_ON,
            BIT_FREQUENCY_OFF,
            BIT_FREQUENCY_NEXT,
            TRANSMIT_START_FREQUENCY,
            TRANSMIT_END_FREQUENCY,
        ];

        for frequency in frequencies.iter() {
            let magnitude = fft_magnitude.get_magnitude(&samples_chunk, *frequency);
            if magnitude >= 0.2 {
                let bin = fft_magnitude.get_frequency_bin(*frequency);
                maximize_bin(&mut complex_samples, bin, sample_size, max_magnitude);
            }
        }
        // println!("\n\n");

        // for bin in 0..sample_size - 1 {
        //     if !bins.contains(&bin) {
        //         silence_bin(&mut complex_samples, bin, sample_size);
        //     }
        // }

        fft_inverse.process(&mut complex_samples);

        for (j, complex) in complex_samples.iter().enumerate() {
            let value: f32 = if complex.re != 0.0 {
                complex.re * (2.0 / sample_size as f32)
            } else {
                0.0
            };

            let value = if value.is_sign_positive() && value > max_magnitude {
                max_magnitude
            } else if value.is_sign_negative() && value < -max_magnitude {
                -max_magnitude
            } else {
                value
            };

            // if value > max_magnitude || value < -max_magnitude {
            //     println!("VALUE: {}", value);
            // }
            samples[i + j] = value;
        }
    }

    print_samples(&samples);
    save_audio("maximized_audio.wav", &samples, &spec);
}
