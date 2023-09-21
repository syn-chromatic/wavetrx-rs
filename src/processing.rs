use std::fs::File;
use std::io::BufReader;

use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use rustfft::FftPlanner;

use hound::{WavReader, WavSpec};

use crate::utils::save_audio;

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
        // complex_samples[sample_size - bin].re = 0.0;
        // complex_samples[sample_size - bin].im = 0.0;
    }
}

fn maximize_bin(
    complex_samples: &mut Vec<Complex<f32>>,
    bin: usize,
    sample_size: usize,
    max_magnitude: f32,
) {
    if bin > 0 && bin != sample_size {
        // complex_samples[bin].re = 1000.0;
        // complex_samples[sample_size - bin].re = 1000.0;
        let complex_bin: Complex<f32> = complex_samples[bin];
        let complex_mirror: Complex<f32> = complex_samples[sample_size - bin];
        let bin_inverse: f32 = complex_bin.inv().norm();
        let mirror_inverse: f32 = complex_mirror.inv().norm();

        if bin_inverse >= 0.0
            && bin_inverse <= 0.1
            && mirror_inverse >= 0.0
            && mirror_inverse <= 0.1
        {
            let complex_norm: f32 = 1.0 - bin_inverse;
            let inverse_norm: f32 = 1.0 - mirror_inverse;

            let bin_scale: f32 = 95.0 / (complex_bin.norm() / max_magnitude);
            let mirror_scale: f32 = 95.0 / (complex_mirror.norm() / max_magnitude);

            complex_samples[bin] = complex_bin.scale(bin_scale);
            complex_samples[sample_size - bin] = complex_mirror.scale(mirror_scale);

            println!(
                "Complex Bin: {} | Inverse Bin: {} | Value: {} | BScale: {} | MScale: {}",
                complex_norm,
                inverse_norm,
                complex_bin.norm(),
                bin_scale,
                mirror_scale,
            );
        } else {
            let bin_scale = 95.0 / (complex_bin.norm() / max_magnitude);
            let mirror_scale = 95.0 / (complex_mirror.norm() / max_magnitude);

            // complex_samples[bin] = complex_bin.scale(0.0);
            // complex_samples[sample_size - bin] = complex_mirror.scale(0.0);

            println!(
                "Complex Bin: {} | Inverse Bin: {} | Value: {} | BScale: {} | MScale: {}",
                0.0,
                0.0,
                complex_bin.norm(),
                bin_scale,
                mirror_scale,
            );
        }

        // complex_samples[bin] = complex_samples[bin].scale(1000.0);
        // complex_samples[sample_size - bin] = complex_samples[sample_size - bin].scale(1000.0);
    }
}

fn get_bin(target_frequency: f32, sample_size: usize, sample_rate: usize) -> usize {
    let bin: usize = (target_frequency * (sample_size as f32) / sample_rate as f32) as usize;
    bin
}

#[test]
fn test_func() {
    let filename = "test1.wav";

    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let spec: WavSpec = reader.spec();
    let audio_bps: u16 = spec.bits_per_sample;
    let max_magnitude: f32 = ((2i32.pow(audio_bps as u32 - 1)) - 1) as f32;
    let sample_rate = spec.sample_rate as usize;
    println!("Max Magnitude: {}", max_magnitude);

    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let mut samples: Vec<f32> = samples.iter().map(|&sample| sample as f32 - 1.0).collect();
    println!("Samples: {}", samples.len());

    // for sample in samples.iter() {
    //     if *sample >= max_magnitude || *sample <= -max_magnitude {
    //         println!("VALUE: {}", sample);
    //     }
    // }
    // return;

    print_samples(&samples);

    // let mut complex_samples: Vec<Complex<f32>> = samples
    //     .iter()
    //     .map(|&sample| Complex::from(sample as f32))
    //     .collect();

    let sample_size: usize = (192_000 * 1000) as usize / 1_000_000;
    // let gap_size: usize = (192_000 * 500) as usize / 1_000_000;
    println!("Sample Size: {}", sample_size);

    let target_frequency1: f32 = 10_000.0;
    let target_frequency2: f32 = 12_000.0;
    let target_frequency3: f32 = 14_000.0;

    let target_frequency4: f32 = 15_000.0;
    let target_frequency5: f32 = 16_000.0;

    let mut planner: FftPlanner<f32> = FftPlanner::new();
    let fft_forward = planner.plan_fft_forward(sample_size);
    let fft_inverse = planner.plan_fft_inverse(sample_size);
    let mut scratch: Vec<Complex<f32>> = get_scratch_space(sample_size);

    for i in (0..samples.len() - sample_size + 1).step_by(sample_size) {
        let samples_chunk: &[f32] = &samples[i..i + sample_size];
        let mut complex_samples: Vec<Complex<f32>> = to_complex_samples(samples_chunk);

        fft_forward.process_with_scratch(&mut complex_samples, &mut scratch);

        let bin1: usize = get_bin(target_frequency1, sample_size, sample_rate);
        let bin2: usize = get_bin(target_frequency2, sample_size, sample_rate);
        let bin3: usize = get_bin(target_frequency3, sample_size, sample_rate);
        let bin4: usize = get_bin(target_frequency4, sample_size, sample_rate);
        let bin5: usize = get_bin(target_frequency5, sample_size, sample_rate);

        let bins: Vec<usize> = vec![bin1, bin2, bin3, bin4, bin5];

        for bin in bins.iter() {
            maximize_bin(&mut complex_samples, *bin, sample_size, max_magnitude);
        }
        println!("\n\n");

        // for bin in 0..sample_size - 1 {
        //     if !bins.contains(&bin) {
        //         silence_bin(&mut complex_samples, bin, sample_size);
        //     }
        // }

        fft_inverse.process(&mut complex_samples);

        for (j, complex) in complex_samples.iter().enumerate() {
            let value: f32 = if complex.re != 0.0 {
                complex.re / (sample_size as f32 + 1.0)
            } else {
                0.0
            };

            let value = if value > max_magnitude {
                max_magnitude
            } else if value < -max_magnitude {
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
    save_audio("maximized_audio.wav", &samples, spec);
}
