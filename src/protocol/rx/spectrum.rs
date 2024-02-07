use std::cmp::Ordering;
use std::f32::consts;
use std::sync::Arc;

use hound::WavSpec;
use rustfft::num_complex::Complex;
use rustfft::Fft;
use rustfft::FftPlanner;

use crate::audio::types::SampleEncoding;
use crate::audio::types::SampleSpec;

use crate::utils::get_bit_depth_magnitudes;

pub struct FourierMagnitude {
    fft: Arc<dyn Fft<f32>>,
    sample_rate: usize,
    sample_size: usize,
}

impl FourierMagnitude {
    pub fn new(sample_size: usize, spec: &SampleSpec) -> Self {
        let mut planner: FftPlanner<f32> = FftPlanner::<f32>::new();
        let fft: Arc<dyn Fft<f32>> = planner.plan_fft_forward(sample_size);
        let sample_rate: usize = spec.sample_rate() as usize;

        FourierMagnitude {
            fft,
            sample_size,
            sample_rate,
        }
    }

    pub fn get_magnitude(&self, samples: &[f32], target_frequency: f32) -> f32 {
        let mut buffer: Vec<Complex<f32>> = samples.iter().map(|&s| Complex::new(s, 0.0)).collect();
        self.fft.process(&mut buffer);

        let k: usize = self.get_frequency_bin(target_frequency);
        let normalization_factor: f32 = 2.0 / self.sample_size as f32;
        let magnitude: f32 = (buffer[k].norm_sqr()).sqrt() * normalization_factor;
        let magnitude_db: f32 = 20.0 * magnitude.log10();
        magnitude_db
    }

    pub fn get_frequency_bin(&self, target_frequency: f32) -> usize {
        let sample_rate: f32 = self.sample_rate as f32;
        let sample_size: f32 = self.sample_size as f32;
        let normalized_frequency: f32 = target_frequency / sample_rate;
        let scaled_frequency: f32 = sample_size * normalized_frequency;
        let biased_frequency: f32 = 0.5 + scaled_frequency;
        let k: usize = biased_frequency as usize;
        k
    }

    pub fn get_sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub fn get_sample_size(&self) -> usize {
        self.sample_size
    }
}

pub struct GoertzelMagnitude {
    sample_rate: usize,
    sample_size: usize,
}

impl GoertzelMagnitude {
    pub fn new(sample_size: usize, spec: WavSpec) -> Self {
        let sample_rate: usize = spec.sample_rate as usize;

        GoertzelMagnitude {
            sample_size,
            sample_rate,
        }
    }

    pub fn get_magnitude(&self, samples: &[f32], target_frequency: f32) -> f32 {
        let mut q1: f32 = 0.0;
        let mut q2: f32 = 0.0;

        let sample_size: f32 = samples.len() as f32;
        let k: usize = self.get_frequency_bin(target_frequency);
        let w: f32 = 2.0 * consts::PI * k as f32 / sample_size;
        let cosine: f32 = f32::cos(w);
        let coeff: f32 = 2.0 * cosine;

        for &sample in samples.iter() {
            let q0: f32 = coeff * q1 - q2 + sample as f32;
            q2 = q1;
            q1 = q0;
        }

        let magnitude: f32 = ((q1 * q1) + (q2 * q2) - (q1 * q2 * coeff)).sqrt();
        let normalization_factor: f32 = 2.0 / sample_size;
        let magnitude: f32 = magnitude * normalization_factor;
        let magnitude_db: f32 = 20.0 * magnitude.log10();
        magnitude_db
    }

    pub fn get_frequency_bin(&self, target_frequency: f32) -> usize {
        let sample_rate: f32 = self.sample_rate as f32;
        let sample_size: f32 = self.sample_size as f32;
        let normalized_frequency: f32 = target_frequency / sample_rate;
        let scaled_frequency: f32 = sample_size * normalized_frequency;
        let biased_frequency: f32 = 0.5 + scaled_frequency;
        let k: usize = biased_frequency as usize;
        k
    }

    pub fn get_sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub fn get_sample_size(&self) -> usize {
        self.sample_size
    }
}

pub struct Normalizer<'a> {
    samples: &'a mut [f32],
}

impl<'a> Normalizer<'a> {
    pub fn new(samples: &'a mut [f32]) -> Self {
        Normalizer { samples }
    }

    pub fn normalize(&mut self, bit_depth: usize, min_floor: f32) {
        let (mut positive, mut negative): (f32, f32) = self.find_max_magnitudes();
        let (p_min, n_min): (f32, f32) = self.get_normalization_mins(bit_depth, min_floor);
        self.clamp_max_magnitudes(&mut positive, &mut negative, p_min, n_min);

        for sample in self.samples.iter_mut() {
            if sample.is_normal() {
                if sample.is_sign_positive() {
                    *sample /= positive
                } else if sample.is_sign_negative() {
                    *sample /= negative.abs()
                };
            }
        }
    }

    pub fn re_normalize(&mut self, min_floor: f32) {
        let (mut positive, mut negative): (f32, f32) = self.find_max_magnitudes();
        let (p_min, n_min): (f32, f32) = (min_floor, -min_floor);
        self.clamp_max_magnitudes(&mut positive, &mut negative, p_min, n_min);

        for sample in self.samples.iter_mut() {
            if sample.is_normal() {
                if sample.is_sign_positive() {
                    *sample /= positive
                } else if sample.is_sign_negative() {
                    *sample /= negative.abs()
                };
            }
        }
    }

    pub fn de_normalize(&mut self, bit_depth: usize) {
        let (p_magnitude, n_magnitude): (f32, f32) = get_bit_depth_magnitudes(bit_depth);

        for sample in self.samples.iter_mut() {
            if sample.is_normal() {
                if sample.is_sign_positive() {
                    *sample *= p_magnitude;
                } else if sample.is_sign_negative() {
                    *sample *= n_magnitude.abs();
                }
            }
        }
    }

    pub fn update_samples(&mut self, samples: &'a mut [f32]) {
        self.samples = samples;
    }

    pub fn to_i32(&self) -> Vec<i32> {
        let samples_f32: &[f32] = &*self.samples;
        let samples_i32: Vec<i32> = samples_f32.into_iter().map(|x| *x as i32).collect();
        samples_i32
    }
}

impl<'a> Normalizer<'a> {
    fn get_normalization_mins(&self, bit_depth: usize, min_floor: f32) -> (f32, f32) {
        let (p_magnitude, n_magnitude): (f32, f32) = get_bit_depth_magnitudes(bit_depth);
        let p_min: f32 = p_magnitude * min_floor;
        let n_min: f32 = n_magnitude * min_floor;
        (p_min, n_min)
    }

    fn clamp_max_magnitudes(&self, positive: &mut f32, negative: &mut f32, p_min: f32, n_min: f32) {
        if *positive < p_min {
            *positive = f32::INFINITY;
        }
        if *negative > n_min {
            *negative = f32::NEG_INFINITY;
        }
    }

    fn find_max_magnitudes(&self) -> (f32, f32) {
        let positive_magnitude: &f32 = self
            .samples
            .iter()
            .max_by(Self::positive_comparison)
            .unwrap();
        let negative_magnitude: &f32 = self
            .samples
            .iter()
            .max_by(Self::negative_comparison)
            .unwrap();
        (*positive_magnitude, *negative_magnitude)
    }

    fn positive_comparison(a: &&f32, b: &&f32) -> Ordering {
        match (a.is_sign_positive(), b.is_sign_positive()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (true, true) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (false, false) => Ordering::Equal,
        }
    }

    fn negative_comparison(a: &&f32, b: &&f32) -> Ordering {
        match (a.is_sign_negative(), b.is_sign_negative()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (true, true) => b.partial_cmp(a).unwrap_or(Ordering::Equal),
            (false, false) => Ordering::Equal,
        }
    }
}
