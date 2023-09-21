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
    AUDIO_SR, BIT_FREQUENCY_NEXT, BIT_FREQUENCY_OFF, BIT_FREQUENCY_ON, MAGNITUDE_THRESHOLD,
    SAMPLING_MAGNITUDE, TONE_GAP_US, TONE_LENGTH_US, TRANSMIT_END_FREQUENCY,
    TRANSMIT_START_FREQUENCY,
};

fn tone_magnitude(samples: &[f32], target_frequency: u32) -> f32 {
    let mut q1: f32 = 0.0;
    let mut q2: f32 = 0.0;

    let sample_count: f32 = samples.len() as f32;
    let k: u32 = (0.5 + (sample_count * target_frequency as f32) / AUDIO_SR as f32) as u32;
    let w: f32 = 2.0 * consts::PI * k as f32 / sample_count;
    let cosine: f32 = f32::cos(w);
    let coeff: f32 = 2.0 * cosine;

    for &sample in samples.iter() {
        let q0: f32 = coeff * q1 - q2 + sample as f32;
        q2 = q1;
        q1 = q0;
    }

    let magnitude: f32 = ((q1 * q1) + (q2 * q2) - (q1 * q2 * coeff)).sqrt();
    magnitude / sample_count
}

struct FFTMagnitude {
    fft: Arc<dyn Fft<f32>>,
    sample_rate: usize,
    sample_size: usize,
}

impl FFTMagnitude {
    fn new(sample_size: usize, spec: WavSpec) -> Self {
        let mut planner: FftPlanner<f32> = FftPlanner::<f32>::new();
        let fft: Arc<dyn Fft<f32>> = planner.plan_fft_forward(sample_size);
        let sample_rate: usize = spec.sample_rate as usize;
        FFTMagnitude {
            fft,
            sample_size,
            sample_rate,
        }
    }

    fn get_bin(&self, target_frequency: u32) -> usize {
        let target_frequency: f32 = target_frequency as f32;
        let sample_size: f32 = self.sample_size as f32;
        let sample_rate: f32 = self.sample_rate as f32;
        let bin: usize = ((target_frequency * sample_size) / sample_rate) as usize;
        bin
    }

    fn calculate(&self, samples: &[f32], target_frequency: u32) -> f32 {
        let mut buffer: Vec<Complex<f32>> = samples.iter().map(|&s| Complex::new(s, 0.0)).collect();
        self.fft.process(&mut buffer);

        let bin: usize = self.get_bin(target_frequency);
        let normalization_factor: f32 = 2.0 / self.sample_size as f32;
        let magnitude: f32 = (buffer[bin].norm_sqr()).sqrt() * normalization_factor;
        magnitude
    }

    fn get_sample_size(&self) -> usize {
        self.sample_size
    }
}

fn print_magnitude(
    start_magnitude: f32,
    end_magnitude: f32,
    on_magnitude: f32,
    off_magnitude: f32,
    next_magnitude: f32,
) {
    println!(
        "ST: {} | EN: {} | ON: {} | OFF: {} | NXT: {}",
        start_magnitude, end_magnitude, on_magnitude, off_magnitude, next_magnitude
    );
}

#[derive(Clone, Debug)]
enum States {
    Start,
    End,
    Next,
    On,
    Off,
}

#[derive(Debug)]
struct ReceiverStates {
    selection: Option<States>,
}

impl ReceiverStates {
    fn new() -> Self {
        let selection: Option<States> = None;
        ReceiverStates { selection }
    }

    fn is_same_selection(
        &self,
        selection: &States,
        start_magnitude: f32,
        end_magnitude: f32,
        on_magnitude: f32,
        off_magnitude: f32,
        next_magnitude: f32,
    ) -> bool {
        match selection {
            States::Start => start_magnitude >= MAGNITUDE_THRESHOLD,
            States::End => end_magnitude >= MAGNITUDE_THRESHOLD,
            States::Next => next_magnitude >= MAGNITUDE_THRESHOLD,
            States::On => on_magnitude >= MAGNITUDE_THRESHOLD,
            States::Off => off_magnitude >= MAGNITUDE_THRESHOLD,
        }
    }

    fn get_magnitude_selection(
        &self,
        start_magnitude: f32,
        end_magnitude: f32,
        on_magnitude: f32,
        off_magnitude: f32,
        next_magnitude: f32,
    ) -> Option<States> {
        if start_magnitude >= MAGNITUDE_THRESHOLD {
            return Some(States::Start);
        } else if end_magnitude >= MAGNITUDE_THRESHOLD {
            return Some(States::End);
        } else if on_magnitude >= MAGNITUDE_THRESHOLD {
            return Some(States::On);
        } else if off_magnitude >= MAGNITUDE_THRESHOLD {
            return Some(States::Off);
        } else if next_magnitude >= MAGNITUDE_THRESHOLD {
            return Some(States::Next);
        }
        None
    }

    fn handle_magnitudes(
        &mut self,
        start_magnitude: f32,
        end_magnitude: f32,
        on_magnitude: f32,
        off_magnitude: f32,
        next_magnitude: f32,
    ) -> Option<States> {
        if let Some(current_selection) = &self.selection {
            let magnitude_selection: Option<States> = self.get_magnitude_selection(
                start_magnitude,
                end_magnitude,
                on_magnitude,
                off_magnitude,
                next_magnitude,
            );

            if let Some(magnitude_selection) = magnitude_selection {
                if self.is_same_selection(
                    current_selection,
                    start_magnitude,
                    end_magnitude,
                    on_magnitude,
                    off_magnitude,
                    next_magnitude,
                ) {
                    return None;
                }

                match magnitude_selection {
                    States::Start => self.selection = Some(States::Start),
                    States::End => self.selection = Some(States::End),
                    States::Next => {
                        let result = Some(current_selection.clone());
                        self.selection = Some(States::Next);
                        return result;
                    }
                    States::On => self.selection = Some(States::On),
                    States::Off => self.selection = Some(States::Off),
                }
            }
        } else {
            if start_magnitude >= MAGNITUDE_THRESHOLD {
                self.selection = Some(States::Start);
            }
        }
        None
    }
}

fn get_minimum_chunk_size(target_frequency: u32, num_cycles: u32) -> usize {
    let time_for_one_cycle = 1.0 / target_frequency as f32;
    let chunk_time = num_cycles as f32 * time_for_one_cycle;
    (chunk_time * AUDIO_SR as f32).ceil() as usize
}

fn normalize_samples(samples: &[f32]) -> Vec<f32> {
    let samples: Vec<f32> = samples
        .iter()
        .map(|&sample| sample / SAMPLING_MAGNITUDE)
        .collect();
    samples
}

fn get_starting_index(samples: &[f32], fft_magnitude: &FFTMagnitude) -> Option<usize> {
    let mut some_index: Option<usize> = None;
    let mut some_magnitude: Option<f32> = None;
    let mut tries: usize = 0;
    let max_tries: usize = 4;
    let sample_size: usize = fft_magnitude.get_sample_size();

    for i in 0..(samples.len() - sample_size) {
        let window: &[f32] = &samples[i..(i + sample_size)];
        let magnitude: f32 = fft_magnitude.calculate(window, TRANSMIT_START_FREQUENCY);
        if let Some(index_magnitude) = some_magnitude {
            if magnitude >= index_magnitude {
                // tries = 0;
                some_index = Some(i);
                some_magnitude = Some(magnitude);
            } else {
                if tries == max_tries {
                    break;
                }
                tries += 1;
            }
        } else {
            if magnitude >= MAGNITUDE_THRESHOLD {
                some_index = Some(i);
                some_magnitude = Some(magnitude);
            }
        }
    }
    some_index
}

pub fn receiver(filename: &str) -> Option<Vec<u8>> {
    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let spec: WavSpec = reader.spec();
    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let samples: Vec<f32> = samples.iter().map(|&sample| sample as f32).collect();

    let tone_size: usize = (spec.sample_rate * TONE_LENGTH_US) as usize / 1_000_000;
    let gap_size: usize = (spec.sample_rate * TONE_GAP_US) as usize / 1_000_000;

    println!("Tone Size: {}", tone_size);
    println!("Gap Size: {}", gap_size);

    let fft_magnitude: FFTMagnitude = FFTMagnitude::new(tone_size, spec);

    // let highpass_frequency: f32 = 8_000.0;
    // let lowpass_frequency: f32 = 18_000.0;

    // apply_highpass_filter(&mut samples, highpass_frequency, spec);
    // apply_lowpass_filter(&mut samples, lowpass_frequency, spec);
    // save_audio("processed.wav", &samples, spec);

    let samples: Vec<f32> = normalize_samples(&samples);
    println!("Samples: {}", samples.len());

    let start_index: Option<usize> = get_starting_index(&samples, &fft_magnitude);
    // let start_index = Some(0);

    if let Some(index) = start_index {
        println!("Found start at sample index: {}", index);
        let mut sample_index: usize = index;
        let mut receiver_states: ReceiverStates = ReceiverStates::new();
        let mut bits: Vec<u8> = Vec::new();

        while sample_index + tone_size <= samples.len() {
            let samples: &[f32] = &samples[sample_index..(sample_index + tone_size)];

            let start_magnitude: f32 = fft_magnitude.calculate(samples, TRANSMIT_START_FREQUENCY);
            let end_magnitude: f32 = fft_magnitude.calculate(samples, TRANSMIT_END_FREQUENCY);
            let on_magnitude: f32 = fft_magnitude.calculate(samples, BIT_FREQUENCY_ON);
            let off_magnitude: f32 = fft_magnitude.calculate(samples, BIT_FREQUENCY_OFF);
            let next_magnitude: f32 = fft_magnitude.calculate(samples, BIT_FREQUENCY_NEXT);

            sample_index += tone_size + gap_size;

            let result: Option<States> = receiver_states.handle_magnitudes(
                start_magnitude,
                end_magnitude,
                on_magnitude,
                off_magnitude,
                next_magnitude,
            );

            // println!("Result: {:?}", result);

            print_magnitude(
                start_magnitude,
                end_magnitude,
                on_magnitude,
                off_magnitude,
                next_magnitude,
            );

            if let Some(states) = result {
                match states {
                    States::Start => {}
                    States::End => break,
                    States::Next => {}
                    States::On => bits.push(1),
                    States::Off => bits.push(0),
                }
            }
        }
        return Some(bits);
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
