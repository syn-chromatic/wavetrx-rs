use hound::{WavReader, WavSpec, WavWriter};
use std::fs::File;
use std::io::{BufReader, BufWriter};

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
