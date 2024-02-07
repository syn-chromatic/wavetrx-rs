use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;

use crate::audio::types::SampleSpec;

pub trait Scalar {
    fn to_i32(&self) -> i32;
}

impl Scalar for i32 {
    fn to_i32(&self) -> i32 {
        *self
    }
}

impl Scalar for f32 {
    fn to_i32(&self) -> i32 {
        *self as i32
    }
}

pub trait IntoBitDepth {
    fn into_bit_depth(self) -> u32;
}

impl IntoBitDepth for usize {
    fn into_bit_depth(self) -> u32 {
        self as u32
    }
}

impl IntoBitDepth for WavSpec {
    fn into_bit_depth(self) -> u32 {
        self.bits_per_sample as u32
    }
}

impl IntoBitDepth for &WavSpec {
    fn into_bit_depth(self) -> u32 {
        self.bits_per_sample as u32
    }
}

impl IntoBitDepth for SampleSpec {
    fn into_bit_depth(self) -> u32 {
        self.bits_per_sample() as u32
    }
}

impl IntoBitDepth for &SampleSpec {
    fn into_bit_depth(self) -> u32 {
        self.bits_per_sample() as u32
    }
}

pub fn save_audio<T: Scalar>(filename: &str, samples: &[T], spec: &SampleSpec) {
    let wav_spec: WavSpec = (*spec).into();
    let mut writer: WavWriter<BufWriter<File>> =
        WavWriter::create(filename, wav_spec).expect("Error creating WAV writer");

    for sample in samples {
        writer
            .write_sample(sample.to_i32())
            .expect("Error writing sample");
    }
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
