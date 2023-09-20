use std::f64;
use std::f64::consts;
use std::fs::File;
use std::io::BufReader;

use hound;
use hound::WavReader;

use crate::{
    AUDIO_SAMPLE_RATE, BIT_TONE_FREQUENCY_NEXT, BIT_TONE_FREQUENCY_OFF, BIT_TONE_FREQUENCY_ON,
    MAX_MAGNITUDE, TONE_LENGTH_US, TRANSMISSION_END_FREQUENCY, TRANSMISSION_START_FREQUENCY,
};

fn calculate_tone_magnitude(samples: &[i32], target_frequency: u32) -> f64 {
    let mut q1: f64 = 0.0;
    let mut q2: f64 = 0.0;

    let sample_count: f64 = samples.len() as f64;
    let k: u32 = (0.5 + (sample_count * target_frequency as f64) / AUDIO_SAMPLE_RATE as f64) as u32;
    let w: f64 = 2.0 * consts::PI * k as f64 / sample_count;
    let cosine: f64 = f64::cos(w);
    let coeff: f64 = 2.0 * cosine;

    for &sample in samples.iter() {
        let q0: f64 = coeff * q1 - q2 + sample as f64;
        q2 = q1;
        q1 = q0;
    }

    let magnitude: f64 = ((q1 * q1) + (q2 * q2) - (q1 * q2 * coeff)).sqrt();
    magnitude / (MAX_MAGNITUDE * samples.len() as f64)
}

fn print_magnitude(
    start_magnitude: f64,
    end_magnitude: f64,
    on_magnitude: f64,
    off_magnitude: f64,
    next_magnitude: f64,
) {
    if start_magnitude >= 0.1 {
        println!("Start: {}", start_magnitude);
    }
    if end_magnitude >= 0.1 {
        println!("End: {}", end_magnitude);
    }
    if on_magnitude >= 0.1 {
        println!("On: {}", on_magnitude);
    }
    if off_magnitude >= 0.1 {
        println!("Off: {}", off_magnitude);
    }
    if next_magnitude >= 0.1 {
        println!("Next: {}", next_magnitude);
    }
    println!();
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
        start_magnitude: f64,
        end_magnitude: f64,
        on_magnitude: f64,
        off_magnitude: f64,
        next_magnitude: f64,
    ) -> bool {
        match selection {
            States::Start => start_magnitude >= 0.1,
            States::End => end_magnitude >= 0.1,
            States::Next => next_magnitude >= 0.1,
            States::On => on_magnitude >= 0.1,
            States::Off => off_magnitude >= 0.1,
        }
    }

    fn get_magnitude_selection(
        &self,
        start_magnitude: f64,
        end_magnitude: f64,
        on_magnitude: f64,
        off_magnitude: f64,
        next_magnitude: f64,
    ) -> Option<States> {
        if start_magnitude >= 0.1 {
            return Some(States::Start);
        } else if end_magnitude >= 0.1 {
            return Some(States::End);
        } else if on_magnitude >= 0.1 {
            return Some(States::On);
        } else if off_magnitude >= 0.1 {
            return Some(States::Off);
        } else if next_magnitude >= 0.1 {
            return Some(States::Next);
        }
        None
    }

    fn handle_magnitudes(
        &mut self,
        start_magnitude: f64,
        end_magnitude: f64,
        on_magnitude: f64,
        off_magnitude: f64,
        next_magnitude: f64,
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
            if start_magnitude >= 0.1 {
                self.selection = Some(States::Start);
            }
        }
        None
    }
}

pub fn receiver(filename: &str) -> Vec<u8> {
    let chunk_size: usize = (AUDIO_SAMPLE_RATE as usize) / (TONE_LENGTH_US as usize * 4);
    println!("Chunk Size: {}", chunk_size);

    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();

    let mut sample_index: usize = 0;
    let mut receiver_states: ReceiverStates = ReceiverStates::new();
    let mut bits: Vec<u8> = Vec::new();

    while sample_index + chunk_size <= samples.len() {
        let samples: &[i32] = &samples[sample_index..(sample_index + chunk_size)];

        let start_magnitude: f64 = calculate_tone_magnitude(samples, TRANSMISSION_START_FREQUENCY);
        let end_magnitude: f64 = calculate_tone_magnitude(samples, TRANSMISSION_END_FREQUENCY);
        let on_magnitude: f64 = calculate_tone_magnitude(samples, BIT_TONE_FREQUENCY_ON);
        let off_magnitude: f64 = calculate_tone_magnitude(samples, BIT_TONE_FREQUENCY_OFF);
        let next_magnitude: f64 = calculate_tone_magnitude(samples, BIT_TONE_FREQUENCY_NEXT);

        sample_index += chunk_size;

        let result = receiver_states.handle_magnitudes(
            start_magnitude,
            end_magnitude,
            on_magnitude,
            off_magnitude,
            next_magnitude,
        );

        // println!("Result: {:?}", result);

        // print_magnitude(
        //     start_magnitude,
        //     end_magnitude,
        //     on_magnitude,
        //     off_magnitude,
        //     next_magnitude,
        // );

        if let Some(states) = result {
            match states {
                States::Start => {}
                States::End => {}
                States::Next => {}
                States::On => bits.push(1),
                States::Off => bits.push(0),
            }
        }
    }
    bits
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
