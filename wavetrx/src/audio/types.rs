use std::sync::Arc;
use std::sync::RwLock;

use hound::WavSpec;
use std::collections::LinkedList;

pub struct FrameF32 {
    pub samples: Vec<f32>,
}

impl FrameF32 {
    fn i32_to_f32(sample: i32) -> f32 {
        (sample as f32) / (i32::MAX as f32)
    }
}

impl FrameF32 {
    pub fn new() -> Self {
        let samples: Vec<f32> = Vec::new();
        Self { samples }
    }

    pub fn from_f32(samples: &[f32]) -> Self {
        let samples: Vec<f32> = samples.to_vec();
        Self { samples }
    }

    pub fn from_i32(samples_i32: &[i32]) -> Self {
        let mut samples: Vec<f32> = Vec::with_capacity(samples_i32.len());

        for sample in samples_i32.iter() {
            let sample: f32 = Self::i32_to_f32(*sample);
            samples.push(sample);
        }
        Self { samples }
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
}

pub struct FrameBuffer {
    buffer: RwLock<LinkedList<FrameF32>>,
}

impl FrameBuffer {
    pub fn new() -> Arc<Self> {
        let buffer: RwLock<LinkedList<FrameF32>> = RwLock::new(LinkedList::new());
        Arc::new(Self { buffer })
    }

    pub fn add_frame(self: &Arc<Self>, frame: FrameF32) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            buffer_guard.push_back(frame);
        }
    }

    pub fn take(self: &Arc<Self>) -> Option<FrameF32> {
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

    pub fn add_frame(self: &Arc<Self>, frame: FrameF32) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            for sample in frame.samples {
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
