use std::fs::File;
use std::io::BufReader;

use hound;
use hound::{WavReader, WavSpec};

use crate::filters::FrequencyFilters;
use crate::protocol::ProtocolProfile;
use crate::rx::spectrum::{FourierMagnitude, Normalizer};
use crate::utils::save_audio;

use crate::consts::{DB_THRESHOLD, HP_FILTER, LP_FILTER};

#[derive(Clone, Debug, PartialEq, Eq)]
enum States {
    Start,
    End,
    Next,
    Bit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RxOutput {
    Bit(u8),
    End,
    Error,
}

struct RxMagnitudes {
    start: f32,
    end: f32,
    next: f32,
    high: f32,
    low: f32,
}

impl RxMagnitudes {
    fn new(start: f32, end: f32, next: f32, high: f32, low: f32) -> Self {
        RxMagnitudes {
            start,
            end,
            next,
            high,
            low,
        }
    }

    fn evaluate(&self, state: &States) -> bool {
        match state {
            States::Start => self.within_range(self.start),
            States::End => self.within_range(self.end),
            States::Next => self.within_range(self.next),
            States::Bit => self.within_range(self.high) || self.within_range(self.low),
        }
    }

    fn within_range(&self, value: f32) -> bool {
        value >= -DB_THRESHOLD && value <= DB_THRESHOLD
    }

    fn get_bit(&self) -> u8 {
        if self.high > self.low {
            return 1;
        }
        0
    }
}

#[derive(Debug)]
struct RxStates {
    selection: Option<States>,
    expectation: States,
    end_selection: Option<States>,
    end_expectation: Option<States>,
}

impl RxStates {
    fn new() -> Self {
        let selection: Option<States> = None;
        let expectation: States = States::Start;
        let end_selection: Option<States> = None;
        let end_expectation: Option<States> = None;
        RxStates {
            selection,
            expectation,
            end_selection,
            end_expectation,
        }
    }

    fn set_expectation(&mut self) {
        if self.expectation == States::Start || self.expectation == States::Bit {
            self.selection = Some(self.expectation.clone());
            self.expectation = States::Next;
        } else if self.expectation == States::Next {
            if let Some(selection) = &self.selection {
                if *selection == States::Start || *selection == States::Bit {
                    self.expectation = States::Bit;
                }
            }
        }
    }

    fn evaluate_end(&mut self, magnitudes: &RxMagnitudes) -> bool {
        if self.expectation == States::Bit {
            if let Some(selection) = &self.selection {
                if *selection == States::Bit {
                    if magnitudes.evaluate(&States::End) {
                        self.end_selection = Some(States::End);
                        self.end_expectation = Some(States::Next);
                        return true;
                    }
                }
            }
        }
        false
    }

    fn resolve_end(
        &mut self,
        magnitudes: &RxMagnitudes,
        end_evaluation: bool,
        evaluation: bool,
    ) -> Option<RxOutput> {
        if !end_evaluation {
            if let Some(end_expectation) = &self.end_expectation {
                let end_evaluation = magnitudes.evaluate(end_expectation);
                if end_evaluation && !evaluation {
                    return Some(RxOutput::End);
                } else {
                    self.end_selection = None;
                    self.end_expectation = None;
                }
            }
        }
        None
    }

    fn handle_magnitudes(&mut self, magnitudes: &RxMagnitudes) -> Option<RxOutput> {
        let end_evaluation: bool = self.evaluate_end(magnitudes);
        let evaluation: bool = magnitudes.evaluate(&self.expectation);

        let end_resolve: Option<RxOutput> =
            self.resolve_end(magnitudes, end_evaluation, evaluation);
        if end_resolve.is_some() {
            return end_resolve;
        }

        if evaluation {
            self.set_expectation();

            if self.expectation == States::Next {
                if let Some(selection) = &self.selection {
                    if *selection == States::Bit {
                        let bit: u8 = magnitudes.get_bit();
                        return Some(RxOutput::Bit(bit));
                    }
                }
            }
        } else if !evaluation && !end_evaluation {
            return Some(RxOutput::Error);
        }
        None
    }
}

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
        filters.apply_highpass(highpass_frequency);
        filters.apply_lowpass(lowpass_frequency);
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
        let max_consecutive_fails: usize = 50;

        for idx in 0..(samples.len() - sample_size) {
            let samples_chunk: &mut [f32] = self.get_samples_chunk(samples, idx, sample_size);
            let magnitude: f32 = freq_mag.get_magnitude(samples_chunk, self.profile.start);

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
        let mut rx_states: RxStates = RxStates::new();
        let mut last_output: Option<RxOutput> = None;

        while idx + tsz <= samples.len() {
            let samples_chunk: &mut [f32] = self.get_samples_chunk(samples, idx, tsz);
            let magnitudes: RxMagnitudes = self.get_magnitudes(&samples_chunk, &freq_mag);
            let output: Option<RxOutput> = rx_states.handle_magnitudes(&magnitudes);

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

    fn get_minimum_chunk_size(
        &self,
        target_frequency: usize,
        num_cycles: usize,
        sample_rate: usize,
    ) -> usize {
        let time_for_one_cycle: f32 = 1.0 / target_frequency as f32;
        let chunk_time: f32 = num_cycles as f32 * time_for_one_cycle;
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

    fn normalize_samples(&self, samples: &mut [f32], spec: &WavSpec) {
        let bitrate: usize = spec.bits_per_sample as usize;
        let mut normalizer: Normalizer<'_> = Normalizer::new(samples);
        normalizer.normalize(bitrate, 0.1);
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
    let bitrate: usize = spec.bits_per_sample as usize;
    let mut samples: Vec<f32> = samples.to_vec();
    let mut normalizer: Normalizer<'_> = Normalizer::new(&mut samples);
    normalizer.de_normalize(bitrate);
    let samples: Vec<i32> = normalizer.to_i32();
    save_audio("normalized.wav", &samples, spec);
}
