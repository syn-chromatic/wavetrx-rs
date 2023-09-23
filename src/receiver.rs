use std::cmp::Ordering;
use std::f32::consts;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use hound;
use hound::{WavReader, WavSpec};

use rustfft::num_complex::Complex;
use rustfft::Fft;
use rustfft::FftPlanner;

use crate::filters::{apply_highpass_filter, apply_lowpass_filter};
use crate::utils::save_audio;
use crate::{
    BIT_FREQUENCY_NEXT, BIT_FREQUENCY_OFF, BIT_FREQUENCY_ON, DB_THRESHOLD, HP_FILTER, LP_FILTER,
    TONE_GAP_US, TONE_LENGTH_US, TRANSMIT_END_FREQUENCY, TRANSMIT_START_FREQUENCY,
};

fn tone_magnitude(samples: &[f32], target_frequency: usize, sample_rate: usize) -> f32 {
    let mut q1: f32 = 0.0;
    let mut q2: f32 = 0.0;

    let sample_size: f32 = samples.len() as f32;
    let k: usize = (0.5 + (sample_size * target_frequency as f32) / sample_rate as f32) as usize;
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

pub struct FFTMagnitude {
    fft: Arc<dyn Fft<f32>>,
    sample_rate: usize,
    sample_size: usize,
    bitrate: usize,
}

impl FFTMagnitude {
    pub fn new(sample_size: usize, spec: WavSpec) -> Self {
        let mut planner: FftPlanner<f32> = FftPlanner::<f32>::new();
        let fft: Arc<dyn Fft<f32>> = planner.plan_fft_forward(sample_size);
        let sample_rate: usize = spec.sample_rate as usize;
        let bitrate: usize = spec.bits_per_sample as usize;

        FFTMagnitude {
            fft,
            sample_size,
            sample_rate,
            bitrate,
        }
    }

    pub fn calculate(&self, samples: &[f32], target_frequency: usize) -> f32 {
        let mut buffer: Vec<Complex<f32>> = samples.iter().map(|&s| Complex::new(s, 0.0)).collect();
        self.fft.process(&mut buffer);

        let bin: usize = self.get_bin(target_frequency);
        let normalization_factor: f32 = 2.0 / self.sample_size as f32;
        let magnitude: f32 = (buffer[bin].norm_sqr()).sqrt() * normalization_factor;
        let magnitude_db: f32 = 20.0 * magnitude.log10();
        magnitude_db
    }

    pub fn get_bin(&self, target_frequency: usize) -> usize {
        let target_frequency: f32 = target_frequency as f32;
        let sample_size: f32 = self.sample_size as f32;
        let sample_rate: f32 = self.sample_rate as f32;
        let bin: usize = ((target_frequency * sample_size) / sample_rate) as usize;
        bin
    }

    pub fn get_sample_size(&self) -> usize {
        self.sample_size
    }

    pub fn get_sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub fn get_bitrate(&self) -> usize {
        self.bitrate
    }
}

fn print_magnitude(
    start_magnitude: f32,
    end_magnitude: f32,
    on_magnitude: f32,
    off_magnitude: f32,
    next_magnitude: f32,
) {
    let mut boolean = false;

    if start_magnitude >= -DB_THRESHOLD && start_magnitude <= DB_THRESHOLD {
        print!("Start: {:.2} dB", start_magnitude);
        boolean = true;
    }
    if end_magnitude >= -DB_THRESHOLD && end_magnitude <= DB_THRESHOLD {
        if boolean {
            print!(" | ");
        }
        print!("End: {:.2} dB", end_magnitude);
        boolean = true;
    }
    if on_magnitude >= -DB_THRESHOLD && on_magnitude <= DB_THRESHOLD {
        if boolean {
            print!(" | ");
        }
        print!("On: {:.2} dB", on_magnitude);
        boolean = true;
    }
    if off_magnitude >= -DB_THRESHOLD && off_magnitude <= DB_THRESHOLD {
        if boolean {
            print!(" | ");
        }
        print!("Off: {:.2} dB", off_magnitude);
        boolean = true;
    }
    if next_magnitude >= -DB_THRESHOLD && next_magnitude <= DB_THRESHOLD {
        if boolean {
            print!(" | ");
        }
        print!("Next: {:.2} dB", next_magnitude);
        boolean = true;
    }

    if boolean {
        println!();
    }

    // println!(
    //     "ST: {:.2} dB | EN: {:.2} dB | ON: {:.2} dB | OFF: {:.2} dB | NXT: {:.2} dB",
    //     start_magnitude, end_magnitude, on_magnitude, off_magnitude, next_magnitude
    // );
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum States {
    Start,
    End,
    Next,
    Bit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReceiverOutput {
    Bit(u8),
    End,
    Error,
}

struct ReceiverMagnitudes {
    start: f32,
    end: f32,
    next: f32,
    on: f32,
    off: f32,
}

impl ReceiverMagnitudes {
    fn new(start: f32, end: f32, next: f32, on: f32, off: f32) -> Self {
        ReceiverMagnitudes {
            start,
            end,
            next,
            on,
            off,
        }
    }

    fn evaluate(&self, state: &States) -> bool {
        match state {
            States::Start => self.in_range(self.start),
            States::End => self.in_range(self.end),
            States::Next => self.in_range(self.next),
            States::Bit => self.in_range(self.on) || self.in_range(self.off),
        }
    }

    fn in_range(&self, value: f32) -> bool {
        value >= -DB_THRESHOLD && value <= DB_THRESHOLD
    }

    fn get_bit(&self) -> u8 {
        if self.on > self.off {
            return 1;
        }
        0
    }
}

#[derive(Debug)]
struct ReceiverStates {
    selection: Option<States>,
    expectation: States,
    end_selection: Option<States>,
    end_expectation: Option<States>,
}

impl ReceiverStates {
    fn new() -> Self {
        let selection: Option<States> = None;
        let expectation: States = States::Start;
        let end_selection: Option<States> = None;
        let end_expectation: Option<States> = None;
        ReceiverStates {
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

    fn evaluate_end(&mut self, magnitudes: &ReceiverMagnitudes) -> bool {
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
        magnitudes: &ReceiverMagnitudes,
        end_evaluation: bool,
        evaluation: bool,
    ) -> Option<ReceiverOutput> {
        if !end_evaluation {
            if let Some(end_expectation) = &self.end_expectation {
                let end_evaluation = magnitudes.evaluate(end_expectation);
                if end_evaluation && !evaluation {
                    return Some(ReceiverOutput::End);
                } else {
                    self.end_selection = None;
                    self.end_expectation = None;
                }
            }
        }
        None
    }

    fn handle_magnitudes(&mut self, magnitudes: &ReceiverMagnitudes) -> Option<ReceiverOutput> {
        let end_evaluation: bool = self.evaluate_end(magnitudes);
        let evaluation: bool = magnitudes.evaluate(&self.expectation);

        let end_resolve: Option<ReceiverOutput> =
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
                        return Some(ReceiverOutput::Bit(bit));
                    }
                }
            }
        } else if !evaluation && !end_evaluation {
            return Some(ReceiverOutput::Error);
        }
        None
    }
}

fn get_minimum_chunk_size(target_frequency: usize, num_cycles: usize, sample_rate: usize) -> usize {
    let time_for_one_cycle: f32 = 1.0 / target_frequency as f32;
    let chunk_time: f32 = num_cycles as f32 * time_for_one_cycle;
    (chunk_time * sample_rate as f32).ceil() as usize
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

fn get_max_magnitudes(samples: &[f32]) -> (f32, f32) {
    let positive_magnitude: &f32 = samples.iter().max_by(positive_comparison).unwrap();
    let negative_magnitude: &f32 = samples.iter().max_by(negative_comparison).unwrap();
    (*positive_magnitude, *negative_magnitude)
}

pub fn normalize_samples(samples: &[f32], bitrate: usize) -> Vec<f32> {
    let mut normalized_samples: Vec<f32> = Vec::new();
    let (mut positive, mut negative): (f32, f32) = get_max_magnitudes(samples);
    let max_magnitude: f32 = ((2usize.pow(bitrate as u32 - 1)) - 1) as f32;

    if positive < max_magnitude * 0.1 {
        positive = f32::INFINITY;
    }
    if negative > -max_magnitude * 0.1 {
        negative = f32::NEG_INFINITY;
    }

    println!(
        "Positive: {} | Negative: {} | Max: {}",
        positive, negative, max_magnitude
    );

    for sample in samples.iter() {
        let sample: f32 = if sample.is_sign_positive() {
            *sample / positive
        } else {
            *sample / negative.abs()
        };

        normalized_samples.push(sample);
    }
    normalized_samples
}

pub fn re_normalize_samples(samples: &mut [f32]) {
    let (mut positive, mut negative): (f32, f32) = get_max_magnitudes(samples);

    if positive < 0.1 {
        positive = f32::INFINITY;
    }
    if negative > -0.1 {
        negative = f32::NEG_INFINITY;
    }

    for sample in samples.iter_mut() {
        if sample.is_sign_positive() {
            *sample /= positive
        } else {
            *sample /= negative.abs()
        };
    }
}

pub fn de_normalize_samples(samples: &[f32], bitrate: usize) -> Vec<i32> {
    let mut denormalized_samples: Vec<i32> = Vec::new();
    let max_magnitude: f32 = ((2usize.pow(bitrate as u32 - 1)) - 1) as f32;

    for sample in samples.iter() {
        let denormalized_sample: f32 = sample * max_magnitude;
        denormalized_samples.push(denormalized_sample as i32);
    }
    denormalized_samples
}

fn get_magnitudes(samples: &[f32], fft_magnitude: &FFTMagnitude) -> ReceiverMagnitudes {
    let start_frequency: usize = TRANSMIT_START_FREQUENCY;
    let end_frequency: usize = TRANSMIT_END_FREQUENCY;
    let next_frequency: usize = BIT_FREQUENCY_NEXT;
    let on_frequency: usize = BIT_FREQUENCY_ON;
    let off_frequency: usize = BIT_FREQUENCY_OFF;

    let start_magnitude: f32 = fft_magnitude.calculate(samples, start_frequency);
    let end_magnitude: f32 = fft_magnitude.calculate(samples, end_frequency);
    let next_magnitude: f32 = fft_magnitude.calculate(samples, next_frequency);
    let on_magnitude: f32 = fft_magnitude.calculate(samples, on_frequency);
    let off_magnitude: f32 = fft_magnitude.calculate(samples, off_frequency);

    print_magnitude(
        start_magnitude,
        end_magnitude,
        on_magnitude,
        off_magnitude,
        next_magnitude,
    );

    let magnitudes: ReceiverMagnitudes = ReceiverMagnitudes::new(
        start_magnitude,
        end_magnitude,
        next_magnitude,
        on_magnitude,
        off_magnitude,
    );
    magnitudes
}

fn get_starting_index(samples: &mut [f32], fft_magnitude: &FFTMagnitude) -> Option<usize> {
    let mut some_index: Option<usize> = None;
    let mut some_magnitude: Option<f32> = None;
    let mut tries: usize = 0;
    let max_tries: usize = 50;
    let sample_size: usize = fft_magnitude.get_sample_size();

    for i in 0..(samples.len() - sample_size) {
        let samples_chunk: &mut [f32] = &mut samples[i..(i + sample_size)];
        re_normalize_samples(samples_chunk);
        let magnitude: f32 = fft_magnitude.calculate(samples_chunk, TRANSMIT_START_FREQUENCY);
        if let Some(index_magnitude) = some_magnitude {
            if magnitude >= index_magnitude && magnitude <= DB_THRESHOLD {
                tries = 0;
                some_index = Some(i);
                some_magnitude = Some(magnitude);
            } else {
                if tries == max_tries {
                    break;
                }
                tries += 1;
            }
        } else {
            if magnitude >= -DB_THRESHOLD && magnitude <= DB_THRESHOLD {
                some_index = Some(i);
                some_magnitude = Some(magnitude);
            }
        }
    }
    some_index
}

fn apply_filters(samples: &mut Vec<f32>, spec: WavSpec) {
    let highpass_frequency: f32 = HP_FILTER;
    let lowpass_frequency: f32 = LP_FILTER;

    apply_highpass_filter(samples, highpass_frequency, spec);
    apply_lowpass_filter(samples, lowpass_frequency, spec);
    save_audio("processed.wav", &samples, spec);
}

fn save_normalized(samples: &[f32], spec: WavSpec) {
    let bitrate: usize = spec.bits_per_sample as usize;
    let de_normalized_samples: Vec<i32> = de_normalize_samples(&samples, bitrate);
    save_audio("normalized.wav", &de_normalized_samples, spec);
}

pub fn receiver(filename: &str) -> Option<Vec<u8>> {
    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let spec: WavSpec = reader.spec();
    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let mut samples: Vec<f32> = samples.iter().map(|&sample| sample as f32).collect();
    let sample_rate: usize = spec.sample_rate as usize;
    let bitrate: usize = spec.bits_per_sample as usize;

    let tone_size: usize = (sample_rate * TONE_LENGTH_US) as usize / 1_000_000;
    let gap_size: usize = (sample_rate * TONE_GAP_US) as usize / 1_000_000;

    println!("Samples: {}", samples.len());
    println!("Tone Size: {}", tone_size);
    println!("Gap Size: {}", gap_size);

    apply_filters(&mut samples, spec);

    let mut samples: Vec<f32> = normalize_samples(&samples, bitrate);

    let fft_magnitude: FFTMagnitude = FFTMagnitude::new(tone_size, spec);
    let start_index: Option<usize> = get_starting_index(&mut samples, &fft_magnitude);

    if let Some(mut index) = start_index {
        println!("Found start at sample index: {}", index);
        let mut receiver_states: ReceiverStates = ReceiverStates::new();
        let mut bits: Vec<u8> = Vec::new();

        let mut last_state: Option<ReceiverOutput> = None;
        while index + tone_size <= samples.len() {
            let samples_chunk: &mut [f32] = &mut samples[index..(index + tone_size)];
            re_normalize_samples(samples_chunk);
            let magnitudes: ReceiverMagnitudes = get_magnitudes(samples_chunk, &fft_magnitude);
            let result: Option<ReceiverOutput> = receiver_states.handle_magnitudes(&magnitudes);

            last_state = result.clone();
            index += tone_size + gap_size;

            if let Some(states) = result {
                match states {
                    ReceiverOutput::Bit(bit) => bits.push(bit),
                    ReceiverOutput::End => break,
                    ReceiverOutput::Error => break,
                }
            }
        }
        save_normalized(&samples, spec);
        if let Some(last_state) = last_state {
            if last_state == ReceiverOutput::End {
                return Some(bits);
            }
        }
    }
    None
}

fn bits_to_bytes(bits: &Vec<u8>) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();
    for chunk in bits.chunks(8) {
        let mut byte: u8 = 0u8;
        for (index, &bit) in chunk.iter().enumerate() {
            if bit == 1 {
                byte |= 1 << (7 - index);
            }
        }
        bytes.push(byte);
    }
    bytes
}

pub fn bits_to_string(bits: &Vec<u8>) -> String {
    let bytes: Vec<u8> = bits_to_bytes(bits);
    let string: String = String::from_utf8(bytes).expect("Failed to convert to string");
    string
}
