use crate::protocol::ProtocolProfile;
use crate::rx::receiver::Receiver;
use crate::tx::transmitter::Transmitter;
use crate::utils::bits_to_string;

use crate::consts::{AUDIO_BPS, AUDIO_SR, MIN_FREQ_SEP};
use crate::get_profile;

#[test]
fn test_transmitter() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    let filename: &str = "transmitted_audio.wav";
    let string: &str = "Test String";
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

#[test]
fn test_receiver() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    // let filename: &str = "transmitted_audio.wav";
    let filename: &str = "test7.wav";
    // let filename: &str = "maximized_audio.wav";

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
