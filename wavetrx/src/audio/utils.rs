use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;

use super::types::AudioSpec;

pub trait Scalar {
    fn to_i32(&self) -> i32;
    fn to_f32(&self) -> f32;
}

impl Scalar for i32 {
    fn to_i32(&self) -> i32 {
        *self
    }

    fn to_f32(&self) -> f32 {
        *self as f32
    }
}

impl Scalar for f32 {
    fn to_i32(&self) -> i32 {
        *self as i32
    }
    fn to_f32(&self) -> f32 {
        *self
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

impl IntoBitDepth for AudioSpec {
    fn into_bit_depth(self) -> u32 {
        self.bits_per_sample() as u32
    }
}

impl IntoBitDepth for &AudioSpec {
    fn into_bit_depth(self) -> u32 {
        self.bits_per_sample() as u32
    }
}

pub fn get_bit_depth_magnitudes<T: IntoBitDepth>(source: T) -> (f32, f32) {
    let bit_depth: u32 = source.into_bit_depth();
    let positive_magnitude: f32 = ((2i32.pow(bit_depth - 1)) - 1) as f32;
    let negative_magnitude: f32 = -positive_magnitude - 1.0;
    (positive_magnitude, negative_magnitude)
}

pub fn save_audio<T: Scalar>(filename: &str, samples: &[T], spec: &AudioSpec) {
    let wav_spec: WavSpec = (*spec).into();
    let mut writer: WavWriter<BufWriter<File>> =
        WavWriter::create(filename, wav_spec).expect("Error creating WAV writer");

    match spec.encoding() {
        super::types::SampleEncoding::F32 => {
            for sample in samples {
                writer
                    .write_sample(sample.to_f32())
                    .expect("Error writing sample");
            }
        }
        super::types::SampleEncoding::I32 => {
            for sample in samples {
                writer
                    .write_sample(sample.to_i32())
                    .expect("Error writing sample");
            }
        }
    }
}
