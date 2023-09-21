mod receiver;
mod transmitter;

use crate::receiver::{bits_to_string, receiver};
use crate::transmitter::generate_audio_data;

pub const AUDIO_BITS_PER_SAMPLE: u16 = 16;
pub const AUDIO_SAMPLE_RATE: u32 = 48000;
pub const TONE_LENGTH_US: u32 = 50 * 1000;

pub const BIT_TONE_FREQUENCY_ON: u32 = 10_000;
pub const BIT_TONE_FREQUENCY_OFF: u32 = 12_000;
pub const BIT_TONE_FREQUENCY_NEXT: u32 = 14_000;

pub const TRANSMISSION_START_FREQUENCY: u32 = 15_000;
pub const TRANSMISSION_END_FREQUENCY: u32 = 16_000;

pub const SAMPLE_MAGNITUDE: f32 = ((2i32.pow(AUDIO_BITS_PER_SAMPLE as u32 - 1)) - 1) as f32;
pub const MAGNITUDE_THRESHOLD: f32 = 0.01;

fn main() {}

#[test]
fn test_transmitter() {
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
    let filename: &str = "transmitted_recording1.wav";
    let bits: Option<Vec<u8>> = receiver(filename);
    if let Some(bits) = bits {
        let string: String = bits_to_string(&bits);
        println!("{}", "-".repeat(20));
        println!("Output: {}", string);

        println!();
        for bit in bits {
            print!("{}", bit);
        }
        println!();
        println!("{}", "-".repeat(20));
    }
}