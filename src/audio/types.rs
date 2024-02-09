use std::sync::Arc;
use std::sync::RwLock;

use std::collections::LinkedList;

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
    buffer: RwLock<LinkedList<Vec<f32>>>,
}

impl FrameBuffer {
    pub fn new() -> Arc<Self> {
        let buffer: RwLock<LinkedList<Vec<f32>>> = RwLock::new(LinkedList::new());
        Arc::new(Self { buffer })
    }

    pub fn add(self: &Arc<Self>, frame: Vec<f32>) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            buffer_guard.push_back(frame);
        }
    }

    pub fn take(self: &Arc<Self>) -> Option<Vec<f32>> {
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

    pub fn add(self: &Arc<Self>, sample: f32) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            buffer_guard.push_back(sample);
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
