use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;

use hound::WavReader;
use hound::WavSpec;
use hound::WavWriter;

use crate::audio::spectrum::Normalizer;
use crate::audio::types::AudioSpec;
use crate::audio::types::IntoBitDepth;
use crate::audio::types::SampleEncoding;
use crate::audio::types::Scalar;

use crate::profile::Bits;
use crate::profile::Markers;
use crate::profile::ProtocolProfile;
use crate::profile::Pulses;

use crate::consts::DefaultProfile;

pub fn get_default_profile() -> ProtocolProfile {
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

    let profile: ProtocolProfile = ProtocolProfile::new(markers, bits, pulses);
    profile
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
        SampleEncoding::F32 => {
            for sample in samples {
                writer
                    .write_sample(sample.to_f32())
                    .expect("Error writing sample");
            }
        }
        SampleEncoding::I32 => {
            for sample in samples {
                writer
                    .write_sample(sample.to_i32())
                    .expect("Error writing sample");
            }
        }
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

pub fn save_normalized(samples: &[f32], spec: &AudioSpec) {
    let bit_depth: usize = spec.bits_per_sample() as usize;
    let mut samples: Vec<f32> = samples.to_vec();
    let mut normalizer: Normalizer<'_> = Normalizer::new(&mut samples);
    normalizer.de_normalize(bit_depth);
    let samples: Vec<i32> = normalizer.to_i32();
    save_audio("normalized.wav", &samples, spec);
}

pub fn save_normalized_name(filename: &str, samples: &[f32], spec: &AudioSpec) {
    let bit_depth: usize = spec.bits_per_sample() as usize;
    let mut samples: Vec<f32> = samples.to_vec();
    let mut normalizer: Normalizer<'_> = Normalizer::new(&mut samples);
    normalizer.de_normalize(bit_depth);
    let samples: Vec<i32> = normalizer.to_i32();
    save_audio(filename, &samples, spec);
}

pub fn read_file(filename: &str) -> (Vec<f32>, WavSpec) {
    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let samples: Vec<f32> = samples.iter().map(|&sample| sample as f32).collect();
    let spec: WavSpec = reader.spec();
    (samples, spec)
}

pub fn read_wav_file(file_path: &str) -> (Vec<f32>, WavSpec) {
    let mut reader: WavReader<BufReader<File>> = hound::WavReader::open(file_path).unwrap();
    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let samples: Vec<f32> = samples
        .iter()
        .map(|&s| (s as f32) / (i32::MAX as f32))
        .collect();
    let spec: WavSpec = reader.spec();
    (samples, spec)
}
