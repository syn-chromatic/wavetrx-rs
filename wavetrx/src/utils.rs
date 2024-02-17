use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use hound::WavReader;

use crate::audio::types::AudioSpec;
use crate::audio::types::IntoBitDepth;
use crate::audio::types::NormSamples;
use crate::protocol::profile::Bits;
use crate::protocol::profile::Markers;
use crate::protocol::profile::Profile;
use crate::protocol::profile::Pulses;

use crate::consts::DefaultProfile;

pub fn get_default_profile() -> Profile {
    let markers: Markers = Markers::new(
        DefaultProfile::MARKER_TONE_START,
        DefaultProfile::MARKER_TONE_END,
        DefaultProfile::MARKER_TONE_NEXT,
    );
    let bits: Bits = Bits::new(DefaultProfile::BIT_TONE_HIGH, DefaultProfile::BIT_TONE_LOW);
    let pulses: Pulses = Pulses::new(
        DefaultProfile::PULSE_LENGTH_US,
        DefaultProfile::PULSE_GAP_US,
    );

    let profile: Profile = Profile::new(markers, bits, pulses);
    profile
}

pub fn get_bit_depth_magnitudes<T: IntoBitDepth>(source: T) -> (f32, f32) {
    let bit_depth: u32 = source.into_bit_depth();
    let positive_magnitude: f32 = ((2i32.pow(bit_depth - 1)) - 1) as f32;
    let negative_magnitude: f32 = -positive_magnitude - 1.0;
    (positive_magnitude, negative_magnitude)
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

pub fn read_wav_file<P>(filename: P) -> (NormSamples, AudioSpec)
where
    P: AsRef<Path>,
{
    let mut reader: WavReader<BufReader<File>> = hound::WavReader::open(filename).unwrap();
    let spec: AudioSpec = reader.spec().into();

    let samples_i32: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let samples: NormSamples = NormSamples::from_i32(&samples_i32, &spec);

    (samples, spec)
}
