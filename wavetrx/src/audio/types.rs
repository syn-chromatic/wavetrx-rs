use std::collections::LinkedList;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use std::fs::File;
use std::io::BufWriter;

use hound::WavSpec;
use hound::WavWriter;

use super::filters::FrequencyPass;
use super::spectrum::Normalizer;

use crate::consts::HP_FILTER;
use crate::consts::LP_FILTER;

pub struct NormSamples(pub Vec<f32>);

impl NormSamples {
    fn i32_to_f32(sample: i32, spec: &AudioSpec) -> f32 {
        match spec.bits_per_sample() {
            16 => (sample as f32) / (i16::MAX as f32),
            32 => (sample as f32) / (i32::MAX as f32),
            _ => panic!("Unsupported Bits-Per-Sample while normalizing"),
        }
    }
}

impl NormSamples {
    pub fn new() -> Self {
        let samples: Vec<f32> = Vec::new();
        Self { 0: samples }
    }

    pub fn from_slice(samples: &[f32]) -> Self {
        let samples: Vec<f32> = samples.to_vec();
        Self { 0: samples }
    }

    pub fn from_vec(samples: Vec<f32>) -> Self {
        Self { 0: samples }
    }

    pub fn from_i32(samples_i32: &[i32], spec: &AudioSpec) -> Self {
        let mut samples: Vec<f32> = Vec::with_capacity(samples_i32.len());

        for sample in samples_i32.iter() {
            let sample: f32 = Self::i32_to_f32(*sample, spec);
            samples.push(sample);
        }
        Self { 0: samples }
    }

    pub fn extend(&mut self, samples: &[f32]) {
        self.0.extend(samples);
    }

    pub fn extend_i32(&mut self, samples_i32: &[i32], spec: &AudioSpec) {
        for sample in samples_i32.iter() {
            let sample: f32 = Self::i32_to_f32(*sample, spec);
            self.0.push(sample);
        }
    }

    pub fn save_file<P>(&self, filename: P, spec: &AudioSpec)
    where
        P: AsRef<Path>,
    {
        let wav_spec: WavSpec = (*spec).into();
        let mut writer: WavWriter<BufWriter<File>> =
            WavWriter::create(filename, wav_spec).expect("Error creating WAV writer");

        for sample in self.0.iter() {
            writer.write_sample(*sample).expect("Error writing sample");
        }
    }
}

impl NormSamples {
    pub fn normalize(&mut self, ceiling: f32, floor: f32) {
        let mut normalizer: Normalizer<'_> = Normalizer::new(&mut self.0);
        normalizer.normalize_floor(ceiling, floor);
    }

    pub fn highpass_filter(&mut self, q_value: f32, spec: &AudioSpec) {
        let highpass_frequency: f32 = HP_FILTER;

        let mut filters: FrequencyPass<'_> = FrequencyPass::new(&mut self.0, spec);
        filters.apply_highpass(highpass_frequency, q_value);
    }

    pub fn lowpass_filter(&mut self, q_value: f32, spec: &AudioSpec) {
        let lowpass_frequency: f32 = LP_FILTER;

        let mut filters: FrequencyPass<'_> = FrequencyPass::new(&mut self.0, spec);
        filters.apply_lowpass(lowpass_frequency, q_value);
    }
}

#[derive(Clone, Copy)]
pub enum SampleEncoding {
    F32,
    I32,
}

#[derive(Clone, Copy)]
pub struct AudioSpec {
    sr: u32,
    bps: u16,
    channels: u16,
    encoding: SampleEncoding,
}

impl AudioSpec {
    pub fn new(sr: u32, bps: u16, channels: u16, encoding: SampleEncoding) -> Self {
        Self {
            channels,
            sr,
            bps,
            encoding,
        }
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.sr
    }

    pub fn bits_per_sample(&self) -> u16 {
        self.bps
    }

    pub fn encoding(&self) -> SampleEncoding {
        self.encoding
    }

    pub fn get_magnitudes(&self) -> (i32, i32) {
        let positive_magnitude: i32 = (2i32.pow((self.bps - 1) as u32)) - 1;
        let negative_magnitude: i32 = -positive_magnitude - 1;
        (positive_magnitude, negative_magnitude)
    }

    pub fn sample_timestamp(&self, sample_idx: usize) -> Duration {
        let secs: f32 = sample_idx as f32 / self.sr as f32;
        let nanos: u64 = (secs * 1e9) as u64;
        let timestamp: Duration = Duration::from_nanos(nanos);
        timestamp
    }
}

impl std::fmt::Debug for SampleEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::F32 => write!(f, "F32"),
            Self::I32 => write!(f, "I32"),
        }
    }
}

impl std::fmt::Debug for AudioSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSpec")
            .field("sr", &self.sr)
            .field("bps", &self.bps)
            .field("channels", &self.channels)
            .field("encoding", &self.encoding)
            .finish()
    }
}

pub struct FrameBuffer {
    buffer: RwLock<LinkedList<NormSamples>>,
}

impl FrameBuffer {
    pub fn new() -> Arc<Self> {
        let buffer: RwLock<LinkedList<NormSamples>> = RwLock::new(LinkedList::new());
        Arc::new(Self { buffer })
    }

    pub fn add_frame(self: &Arc<Self>, frame: NormSamples) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            buffer_guard.push_back(frame);
        }
    }

    pub fn take(self: &Arc<Self>) -> Option<NormSamples> {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            return buffer_guard.pop_front();
        }
        None
    }
}

pub struct SampleBuffer {
    buffer: RwLock<LinkedList<f32>>,
}

impl SampleBuffer {
    pub fn new() -> Arc<Self> {
        let buffer: RwLock<LinkedList<f32>> = RwLock::new(LinkedList::new());
        Arc::new(Self { buffer })
    }

    pub fn add_sample(self: &Arc<Self>, sample: f32) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            buffer_guard.push_back(sample);
        }
    }

    pub fn add_samples(self: &Arc<Self>, samples: NormSamples) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            for sample in samples.0 {
                buffer_guard.push_back(sample);
            }
        }
    }

    pub fn take(self: &Arc<Self>) -> Option<f32> {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            return buffer_guard.pop_front();
        }
        None
    }

    pub fn buffer_empty(self: &Arc<Self>) -> bool {
        if let Ok(buffer_guard) = self.buffer.read() {
            return buffer_guard.is_empty();
        }
        false
    }

    pub fn buffer_len(self: &Arc<Self>) -> usize {
        if let Ok(buffer_guard) = self.buffer.read() {
            return buffer_guard.len();
        }
        0
    }
}

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
