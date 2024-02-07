mod audio;
mod consts;
mod filters;
mod impls;
mod processing;
mod protocol;
mod tests;
mod utils;

use crate::protocol::profile::ProtocolProfile;
use crate::protocol::rx::receiver::Receiver;
use crate::protocol::tx::transmitter::Transmitter;
use crate::utils::bits_to_string;

use crate::consts::{
    AUDIO_BPS, AUDIO_SR, BIT_FREQUENCY_NEXT, BIT_FREQUENCY_OFF, BIT_FREQUENCY_ON, MIN_FREQ_SEP,
    TONE_GAP_US, TONE_LENGTH_US, TRANSMIT_END_FREQUENCY, TRANSMIT_START_FREQUENCY,
};

fn get_profile() -> ProtocolProfile {
    let start: f32 = TRANSMIT_START_FREQUENCY;
    let end: f32 = TRANSMIT_END_FREQUENCY;
    let next: f32 = BIT_FREQUENCY_NEXT;
    let high: f32 = BIT_FREQUENCY_ON;
    let low: f32 = BIT_FREQUENCY_OFF;
    let tone_length: usize = TONE_LENGTH_US;
    let gap_length: usize = TONE_GAP_US;

    let profile: ProtocolProfile =
        ProtocolProfile::new(start, end, next, high, low, tone_length, gap_length);
    profile
}

fn transmitter() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    let filename: &str = "transmitted_audio.wav";
    let string = "Test String".repeat(2);
    let data: &[u8] = string.as_bytes();

    println!("Data: {:?}", data);

    let profile: ProtocolProfile = get_profile();
    let sample_rate: usize = AUDIO_SR;
    let bit_depth: usize = AUDIO_BPS;

    let transmitter: Transmitter = Transmitter::new(profile, sample_rate, bit_depth);
    let result: Result<(), hound::Error> = transmitter.create_file(filename, data);

    if let Err(err) = result {
        println!("Error: Failed to generate data: {:?}", err);
        return;
    }

    println!("Generated {} bytes", data.len());
}

fn receiver() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    let filename: &str = "transmitted_audio.wav";
    let profile: ProtocolProfile = get_profile();
    let receiver: Receiver = Receiver::new(profile);
    let bits: Option<Vec<u8>> = receiver.from_file(filename);

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

fn main() {
    println!("Transmitting..");
    transmitter();
    println!("\n");

    println!("Receiving..");
    receiver();
}
