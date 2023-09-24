use std::fs::File;
use std::io::BufReader;

use hound;
use hound::{WavReader, WavSpec};

use crate::filters::FrequencyFilters;
use crate::protocol::ProtocolProfile;
use crate::rx::resolver::RxResolver;
use crate::rx::spectrum::{FourierMagnitude, Normalizer};
use crate::rx::states::{RxMagnitudes, RxOutput};
use crate::utils::save_audio;

use crate::consts::{DB_THRESHOLD, HP_FILTER, LP_FILTER};

pub struct Receiver {
    profile: ProtocolProfile,
}

impl Receiver {
    pub fn new(profile: ProtocolProfile) -> Self {
        Receiver { profile }
    }

    pub fn from_file(&self, filename: &str) -> Option<Vec<u8>> {
        let (mut samples, spec) = self.read_file(filename);
        let tsz: usize = self.get_tone_sample_size(&spec);
        let gsz: usize = self.get_gap_sample_size(&spec);

        println!("Samples: {}", samples.len());
        println!("Tone Sample Size: {}", tsz);
        println!("Gap Sample Size: {}", gsz);

        self.apply_frequency_filters(&mut samples, &spec);
        self.normalize_samples(&mut samples, &spec);

        let freq_mag: FourierMagnitude = FourierMagnitude::new(tsz, spec);
        let start_index: Option<usize> = self.find_starting_index(&mut samples, tsz, &freq_mag);

        if let Some(idx) = start_index {
            println!("Found start at sample index: {}", idx);
            let (bits, output): (Vec<u8>, Option<RxOutput>) =
                self.receive_bits(idx, tsz, gsz, &mut samples, &freq_mag);

            save_normalized(&samples, &spec);
            if let Some(output) = output {
                if output == RxOutput::End {
                    return Some(bits);
                }
            }
        }
        None
    }
}

impl Receiver {
    fn apply_frequency_filters(&self, samples: &mut [f32], spec: &WavSpec) {
        let highpass_frequency: f32 = HP_FILTER;
        let lowpass_frequency: f32 = LP_FILTER;

        let mut filters: FrequencyFilters<'_> = FrequencyFilters::new(samples, spec);
        filters.apply_highpass(highpass_frequency, 0.707);
        filters.apply_lowpass(lowpass_frequency, 0.707);
        save_audio("processed.wav", &samples, spec);
    }

    fn find_starting_index(
        &self,
        samples: &mut [f32],
        sample_size: usize,
        freq_mag: &FourierMagnitude,
    ) -> Option<usize> {
        let mut current_best_idx: Option<usize> = None;
        let mut current_best_magnitude: Option<f32> = None;
        let mut consecutive_fails: usize = 0;
        let max_consecutive_fails: usize = 10;

        let mut idx: usize = 0;
        let skip_cycles: usize = 16;
        let sample_rate: usize = freq_mag.get_sample_rate();

        while idx < (samples.len() - sample_size) {
            let samples_chunk: Vec<f32> = self.get_owned_samples_chunk(samples, idx, sample_size);
            let magnitude: f32 = freq_mag.get_magnitude(&samples_chunk, self.profile.start);

            let terminate: bool = self.update_starting_index_search(
                idx,
                magnitude,
                &mut current_best_idx,
                &mut current_best_magnitude,
                &mut consecutive_fails,
                max_consecutive_fails,
            );
            if terminate {
                break;
            }
            self.update_starting_index(&mut idx, skip_cycles, sample_rate, &current_best_magnitude);
        }
        current_best_idx
    }

    fn update_starting_index_search(
        &self,
        idx: usize,
        magnitude: f32,
        current_best_idx: &mut Option<usize>,
        current_best_magnitude: &mut Option<f32>,
        consecutive_fails: &mut usize,
        max_consecutive_fails: usize,
    ) -> bool {
        match current_best_magnitude {
            Some(previous_best_magnitude) => {
                if magnitude >= *previous_best_magnitude && magnitude <= DB_THRESHOLD {
                    *consecutive_fails = 0;
                    *current_best_idx = Some(idx);
                    *current_best_magnitude = Some(magnitude);
                } else {
                    if *consecutive_fails == max_consecutive_fails {
                        return true;
                    }
                    *consecutive_fails += 1;
                }
            }
            None => {
                if magnitude >= -DB_THRESHOLD && magnitude <= DB_THRESHOLD {
                    *current_best_idx = Some(idx);
                    *current_best_magnitude = Some(magnitude);
                }
            }
        }
        false
    }

    fn update_starting_index(
        &self,
        idx: &mut usize,
        cycles: usize,
        sample_rate: usize,
        current_best_magnitude: &Option<f32>,
    ) {
        if current_best_magnitude.is_none() {
            let frequency: f32 = self.profile.start;
            let idx_skip: usize = self.get_minimum_chunk_size(frequency, cycles, sample_rate);
            *idx += idx_skip;
        } else {
            *idx += 1;
        }
    }

    fn read_file(&self, filename: &str) -> (Vec<f32>, WavSpec) {
        let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
        let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
        let samples: Vec<f32> = samples.iter().map(|&sample| sample as f32).collect();
        let spec: WavSpec = reader.spec();
        (samples, spec)
    }

    fn receive_bits(
        &self,
        mut idx: usize,
        tsz: usize,
        gsz: usize,
        samples: &mut [f32],
        freq_mag: &FourierMagnitude,
    ) -> (Vec<u8>, Option<RxOutput>) {
        let mut bits: Vec<u8> = Vec::new();
        let mut resolver: RxResolver = RxResolver::new();
        let mut last_output: Option<RxOutput> = None;

        while idx + tsz <= samples.len() {
            let samples_chunk: &mut [f32] = self.get_samples_chunk(samples, idx, tsz);
            let magnitudes: RxMagnitudes = self.get_magnitudes(&samples_chunk, &freq_mag);
            let output: Option<RxOutput> = resolver.resolve(&magnitudes);

            last_output = output.clone();
            idx += tsz + gsz;

            if let Some(states) = output {
                match states {
                    RxOutput::Bit(bit) => bits.push(bit),
                    RxOutput::End => break,
                    RxOutput::Error => break,
                }
            }
        }
        (bits, last_output)
    }

    fn get_tone_sample_size(&self, spec: &WavSpec) -> usize {
        let sample_rate: usize = spec.sample_rate as usize;
        let tone_sample_size: usize = (sample_rate * self.profile.tone_length) as usize / 1_000_000;
        tone_sample_size
    }

    fn get_gap_sample_size(&self, spec: &WavSpec) -> usize {
        let sample_rate: usize = spec.sample_rate as usize;
        let gap_sample_size: usize = (sample_rate * self.profile.gap_length) as usize / 1_000_000;
        gap_sample_size
    }

    fn get_magnitudes(&self, samples: &[f32], freq_mag: &FourierMagnitude) -> RxMagnitudes {
        let start_magnitude: f32 = freq_mag.get_magnitude(samples, self.profile.start);
        let end_magnitude: f32 = freq_mag.get_magnitude(samples, self.profile.end);
        let next_magnitude: f32 = freq_mag.get_magnitude(samples, self.profile.next);
        let high_magnitude: f32 = freq_mag.get_magnitude(samples, self.profile.high);
        let low_magnitude: f32 = freq_mag.get_magnitude(samples, self.profile.low);

        let magnitudes: RxMagnitudes = RxMagnitudes::new(
            start_magnitude,
            end_magnitude,
            next_magnitude,
            high_magnitude,
            low_magnitude,
        );

        print_magnitude(&magnitudes);
        magnitudes
    }

    fn get_minimum_chunk_size(&self, frequency: f32, cycles: usize, sample_rate: usize) -> usize {
        let time_for_one_cycle: f32 = 1.0 / frequency;
        let chunk_time: f32 = cycles as f32 * time_for_one_cycle;
        (chunk_time * sample_rate as f32).ceil() as usize
    }

    fn get_samples_chunk<'a>(
        &self,
        samples: &'a mut [f32],
        idx: usize,
        sample_size: usize,
    ) -> &'a mut [f32] {
        let samples_chunk: &mut [f32] = &mut samples[idx..(idx + sample_size)];
        self.re_normalize_samples_chunk(samples_chunk);
        samples_chunk
    }

    fn get_owned_samples_chunk<'a>(
        &self,
        samples: &'a [f32],
        idx: usize,
        sample_size: usize,
    ) -> Vec<f32> {
        let mut samples_chunk: Vec<f32> = samples[idx..(idx + sample_size)].to_vec();
        self.re_normalize_samples_chunk(&mut samples_chunk);
        samples_chunk
    }

    fn normalize_samples(&self, samples: &mut [f32], spec: &WavSpec) {
        let bit_depth: usize = spec.bits_per_sample as usize;
        let mut normalizer: Normalizer<'_> = Normalizer::new(samples);
        normalizer.normalize(bit_depth, 0.1);
    }

    fn re_normalize_samples_chunk(&self, chunk: &mut [f32]) {
        let mut normalizer: Normalizer<'_> = Normalizer::new(chunk);
        normalizer.re_normalize(0.1);
    }
}

fn print_magnitude(magnitudes: &RxMagnitudes) {
    let mut boolean: bool = false;

    if magnitudes.within_range(magnitudes.start) {
        print!("Start: {:.2} dB", magnitudes.start);
        boolean = true;
    }
    if magnitudes.within_range(magnitudes.end) {
        if boolean {
            print!(" | ");
        }
        print!("End: {:.2} dB", magnitudes.end);
        boolean = true;
    }
    if magnitudes.within_range(magnitudes.high) {
        if boolean {
            print!(" | ");
        }
        print!("High: {:.2} dB", magnitudes.high);
        boolean = true;
    }
    if magnitudes.within_range(magnitudes.low) {
        if boolean {
            print!(" | ");
        }
        print!("Low: {:.2} dB", magnitudes.low);
        boolean = true;
    }
    if magnitudes.within_range(magnitudes.next) {
        if boolean {
            print!(" | ");
        }
        print!("Next: {:.2} dB", magnitudes.next);
        boolean = true;
    }

    if boolean {
        println!();
    }
}

fn save_normalized(samples: &[f32], spec: &WavSpec) {
    let bit_depth: usize = spec.bits_per_sample as usize;
    let mut samples: Vec<f32> = samples.to_vec();
    let mut normalizer: Normalizer<'_> = Normalizer::new(&mut samples);
    normalizer.de_normalize(bit_depth);
    let samples: Vec<i32> = normalizer.to_i32();
    save_audio("normalized.wav", &samples, spec);
}
