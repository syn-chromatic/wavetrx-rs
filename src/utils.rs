use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;

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

pub fn save_audio<T: Scalar>(filename: &str, samples: &[T], spec: &WavSpec) {
    let mut writer: WavWriter<BufWriter<File>> =
        WavWriter::create(filename, spec.clone()).expect("Error creating WAV writer");

    for sample in samples {
        writer
            .write_sample(sample.to_i32())
            .expect("Error writing sample");
    }
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
