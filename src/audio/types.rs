use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

#[derive(Clone, Copy)]
pub enum SampleEncoding {
    Float,
    Int,
}

#[derive(Clone, Copy)]
pub struct SampleSpec {
    sr: u32,
    bps: u16,
    channels: u16,
    encoding: SampleEncoding,
}

impl SampleSpec {
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

#[derive(Copy, Clone)]
pub enum BufferState {
    Available,
    Used,
}

pub struct BufferSample {
    sample: Vec<f32>,
    state: Mutex<BufferState>,
}

impl BufferSample {
    pub fn new(sample: Vec<f32>) -> Self {
        let state: Mutex<BufferState> = Mutex::new(BufferState::Available);
        Self { sample, state }
    }

    pub fn state(&self) -> BufferState {
        *self.state.lock().unwrap()
    }

    pub fn take(&self) -> Vec<f32> {
        *self.state.lock().unwrap() = BufferState::Used;
        self.sample.clone()
    }
}

pub struct Buffer {
    index: RwLock<usize>,
    buffer: RwLock<Vec<BufferSample>>,
}

impl Buffer {
    fn read_index(&self) -> Option<usize> {
        if let Ok(index_guard) = self.index.read() {
            return Some(*index_guard);
        }
        None
    }

    fn write_index(&self, index: usize) {
        if let Ok(mut index_guard) = self.index.write() {
            *index_guard = index;
        }
    }
}

impl Buffer {
    pub fn new() -> Arc<Self> {
        let index: RwLock<usize> = RwLock::new(0);
        let buffer: RwLock<Vec<BufferSample>> = RwLock::new(Vec::new());
        Arc::new(Self { index, buffer })
    }

    pub fn add(self: &Arc<Self>, sample: Vec<f32>) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            (*buffer_guard).push(BufferSample::new(sample));
            if let Some(index) = self.read_index() {
                buffer_guard.drain(0..index);
            }
            self.write_index(0);
        }
    }

    pub fn take(self: &Arc<Self>) -> Option<Vec<f32>> {
        if let Ok(buffer_guard) = self.buffer.read() {
            if let Some(index) = self.read_index() {
                if index < buffer_guard.len() {
                    let buffer_sample: &BufferSample = &buffer_guard[index as usize];
                    match buffer_sample.state() {
                        BufferState::Available => {
                            let sample: Vec<f32> = buffer_sample.take();
                            return Some(sample);
                        }
                        BufferState::Used => {
                            self.write_index(index + 1);
                        }
                    }
                }
            }
        }
        None
    }
}
