use std::cmp::Ordering;
use std::f32::consts;
use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::Fft;
use rustfft::FftPlanner;

use crate::audio::types::AudioSpec;
use crate::protocol::profile::SizedPulses;

pub struct FourierMagnitude {
    fft: Arc<dyn Fft<f32>>,
    pulses: SizedPulses,
    spec: AudioSpec,
}

impl FourierMagnitude {
    pub fn new(pulses: &SizedPulses, spec: &AudioSpec) -> Self {
        let pulses: SizedPulses = pulses.clone();
        let spec: AudioSpec = spec.clone();

        let mut planner: FftPlanner<f32> = FftPlanner::<f32>::new();
        let fft: Arc<dyn Fft<f32>> = planner.plan_fft_forward(pulses.tone_size());

        FourierMagnitude { fft, pulses, spec }
    }

    pub fn get_magnitude(&self, samples: &[f32], target_frequency: f32) -> f32 {
        let mut buffer: Vec<Complex<f32>> = samples.iter().map(|&s| Complex::new(s, 0.0)).collect();
        self.fft.process(&mut buffer);

        let k: usize = self.get_frequency_bin(target_frequency);
        let normalization_factor: f32 = 2.0 / self.pulses.tone_size() as f32;
        let magnitude: f32 = (buffer[k].norm_sqr()).sqrt() * normalization_factor;
        let magnitude_db: f32 = 20.0 * magnitude.log10();
        magnitude_db
    }

    pub fn get_frequency_bin(&self, target_frequency: f32) -> usize {
        let sample_rate: f32 = self.spec.sample_rate() as f32;
        let sample_size: f32 = self.pulses.tone_size() as f32;
        let normalized_frequency: f32 = target_frequency / sample_rate;
        let scaled_frequency: f32 = sample_size * normalized_frequency;
        let biased_frequency: f32 = 0.5 + scaled_frequency;
        let k: usize = biased_frequency as usize;
        k
    }
}

pub struct GoertzelMagnitude {
    pulses: SizedPulses,
    spec: AudioSpec,
}

impl GoertzelMagnitude {
    pub fn new(pulses: &SizedPulses, spec: &AudioSpec) -> Self {
        let pulses: SizedPulses = pulses.clone();
        let spec: AudioSpec = spec.clone();

        GoertzelMagnitude { pulses, spec }
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
        let sample_rate: f32 = self.spec.sample_rate() as f32;
        let sample_size: f32 = self.pulses.tone_size() as f32;
        let normalized_frequency: f32 = target_frequency / sample_rate;
        let scaled_frequency: f32 = sample_size * normalized_frequency;
        let biased_frequency: f32 = 0.5 + scaled_frequency;
        let k: usize = biased_frequency as usize;
        k
    }
}

pub struct Normalizer<'a> {
    samples: &'a mut [f32],
}

impl<'a> Normalizer<'a> {
    pub fn new(samples: &'a mut [f32]) -> Self {
        Normalizer { samples }
    }

    pub fn normalize(&mut self, ceiling: f32) {
        let (mut p_max, mut n_max): (f32, f32) = self.find_max_magnitudes();
        let (p_min, n_min): (f32, f32) = (0.0, 0.0);

        p_max /= ceiling;
        n_max /= ceiling;

        self.normalize_samples(p_max, n_max, p_min, n_min);
    }

    pub fn normalize_floor(&mut self, ceiling: f32, floor: f32) {
        let (mut p_max, mut n_max): (f32, f32) = self.find_max_magnitudes();
        let (p_min, n_min): (f32, f32) = (floor, -floor);

        p_max /= ceiling;
        n_max /= ceiling;

        self.normalize_samples(p_max, n_max, p_min, n_min);
    }
}

impl<'a> Normalizer<'a> {
    fn normalize_samples(&mut self, p_max: f32, n_max: f32, p_min: f32, n_min: f32) {
        for sample in self.samples.iter_mut() {
            if sample.is_normal() {
                if sample.is_sign_positive() {
                    Self::normalize_positive(sample, p_max, p_min);
                } else if sample.is_sign_negative() {
                    Self::normalize_negative(sample, n_max, n_min);
                };
            }
        }
    }

    fn compare_positive(a: &&f32, b: &&f32) -> Ordering {
        match (a.is_sign_positive(), b.is_sign_positive()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (true, true) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (false, false) => Ordering::Equal,
        }
    }

    fn compare_negative(a: &&f32, b: &&f32) -> Ordering {
        match (a.is_sign_negative(), b.is_sign_negative()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (true, true) => b.partial_cmp(a).unwrap_or(Ordering::Equal),
            (false, false) => Ordering::Equal,
        }
    }

    fn normalize_positive(sample: &mut f32, p_max: f32, p_min: f32) {
        if *sample < p_min {
            *sample = 0.0;
        } else {
            *sample /= p_max
        }
    }

    fn normalize_negative(sample: &mut f32, n_max: f32, n_min: f32) {
        if *sample > n_min {
            *sample = 0.0;
        } else {
            *sample /= n_max.abs();
        }
    }

    fn find_max_magnitudes(&self) -> (f32, f32) {
        let p_max: &f32 = self.samples.iter().max_by(Self::compare_positive).unwrap();
        let n_max: &f32 = self.samples.iter().max_by(Self::compare_negative).unwrap();
        (*p_max, *n_max)
    }
}

#[test]
fn test_normalizer() {
    use super::types::NormSamples;
    use super::types::SampleEncoding;
    use hound::WavReader;
    use std::fs::File;
    use std::io::BufReader;

    let filename: &str = "two_tone.wav";
    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let spec: AudioSpec = reader.spec().into();

    println!("{:?}", spec);

    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let mut samples: NormSamples = NormSamples::from_i32(&samples, &spec);

    let mut normalizer: Normalizer<'_> = Normalizer::new(&mut samples.0);
    normalizer.normalize_floor(0.9, 0.85);

    let spec: AudioSpec =
        AudioSpec::new(spec.sample_rate(), 32, spec.channels(), SampleEncoding::F32);
    samples.save_file("test_normalizer.wav", &spec);
}
