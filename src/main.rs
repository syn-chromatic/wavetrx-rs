mod audio;
mod consts;
mod processing;
mod protocol;
mod tests;

use std::time::Duration;

use crate::protocol::profile::Bits;
use crate::protocol::profile::Markers;
use crate::protocol::profile::ProtocolProfile;
use crate::protocol::profile::Pulses;

use crate::protocol::rx::receiver::Receiver;
use crate::protocol::tx::transmitter::Transmitter;
use crate::protocol::utils::bits_to_string;

use crate::consts::{
    AUDIO_BPS, AUDIO_SR, BIT_TONE_HIGH, BIT_TONE_LOW, MARKER_TONE_END, MARKER_TONE_NEXT,
    MARKER_TONE_START, MIN_FREQ_SEP, PULSE_GAP_US, PULSE_LENGTH_US,
};

fn get_profile() -> ProtocolProfile {
    let markers: Markers = Markers::new(MARKER_TONE_START, MARKER_TONE_END, MARKER_TONE_NEXT);
    let bits: Bits = Bits::new(BIT_TONE_HIGH, BIT_TONE_LOW);
    let pulses: Pulses = Pulses::new(PULSE_LENGTH_US, PULSE_GAP_US);

    let profile: ProtocolProfile = ProtocolProfile::new(markers, bits, pulses);
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
