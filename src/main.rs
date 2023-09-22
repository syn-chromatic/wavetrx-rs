mod filters;
mod processing;
mod receiver;
mod transmitter;
mod utils;

use crate::receiver::{bits_to_string, receiver};
use crate::transmitter::generate_audio_data;

pub const AUDIO_BPS: usize = 32;
pub const AUDIO_SR: usize = 48_000;
pub const TONE_LENGTH_US: usize = 10_000;
pub const TONE_GAP_US: usize = 10_000;

pub const SAMPLE_SIZE: f32 = (AUDIO_SR as f32 * TONE_LENGTH_US as f32) / 1_000_000.0;
pub const MIN_FREQ_SEP: f32 = AUDIO_SR as f32 / SAMPLE_SIZE;

pub const LP_FILTER: f32 = 19_000.0;
pub const HP_FILTER: f32 = 16_000.0;

pub const BIT_FREQUENCY_ON: usize = 19_000;
pub const BIT_FREQUENCY_OFF: usize = 19_200;
pub const BIT_FREQUENCY_NEXT: usize = 19_400;

pub const TRANSMIT_START_FREQUENCY: usize = 19_600;
pub const TRANSMIT_END_FREQUENCY: usize = 19_800;

pub const SAMPLING_MAGNITUDE: f32 = ((2usize.pow(AUDIO_BPS as u32 - 1)) - 1) as f32;
pub const DB_THRESHOLD: f32 = 15.0;

fn main() {}

#[test]
fn test_transmitter() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    let filename: &str = "transmitted_audio.wav";
    let string: &str = "Test String";
    let data: &[u8] = string.as_bytes();

    println!("Data: {:?}", data);
    if let Err(err) = generate_audio_data(filename, data) {
        println!("Error: Failed to generate data: {:?}", err);
        return;
    }

    println!("Generated {} bytes", data.len());
}

#[test]
fn test_receiver() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    // let filename: &str = "transmitted_audio.wav";
    let filename: &str = "test7.wav";
    // let filename: &str = "maximized_audio.wav";
    let bits: Option<Vec<u8>> = receiver(filename);
    if let Some(bits) = bits {
        println!("{}", "-".repeat(20));
        println!();
        for bit in bits.iter() {
            print!("{}", bit);
        }
        println!();

        let string: String = bits_to_string(&bits);
        println!("Decoded: {}", string);
        println!();
        println!("{}", "-".repeat(20));
    }
}
