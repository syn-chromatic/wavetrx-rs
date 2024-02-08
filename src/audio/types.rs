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
pub enum ItemState {
    Available,
    Used,
    Empty,
}

pub struct Frame {
    frame: Vec<f32>,
    state: Mutex<ItemState>,
}

impl Frame {
    pub fn new(frame: Vec<f32>) -> Self {
        let state: Mutex<ItemState> = Mutex::new(ItemState::Available);
        Self { frame, state }
    }

    pub fn state(&self) -> ItemState {
        *self.state.lock().unwrap()
    }

    pub fn take(&self) -> Vec<f32> {
        *self.state.lock().unwrap() = ItemState::Used;
        self.frame.clone()
    }
}

pub struct FrameBuffer {
    index: RwLock<usize>,
    buffer: RwLock<Vec<Frame>>,
}

impl FrameBuffer {
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

impl FrameBuffer {
    pub fn new() -> Arc<Self> {
        let index: RwLock<usize> = RwLock::new(0);
        let buffer: RwLock<Vec<Frame>> = RwLock::new(Vec::new());
        Arc::new(Self { index, buffer })
    }

    pub fn add(self: &Arc<Self>, sample: Vec<f32>) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            (*buffer_guard).push(Frame::new(sample));

            // This could take a hit on performance
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
                    let buffer_sample: &Frame = &buffer_guard[index as usize];
                    match buffer_sample.state() {
                        ItemState::Available => {
                            let sample: Vec<f32> = buffer_sample.take();
                            return Some(sample);
                        }
                        ItemState::Used => {
                            self.write_index(index + 1);
                        }
                        _ => {}
                    }
                }
            }
        }
        None
    }
}

#[derive(Copy, Clone)]
pub struct Sample {
    sample: f32,
    state: ItemState,
}

impl Sample {
    pub fn new(sample: f32) -> Self {
        let state: ItemState = ItemState::Available;
        Self { sample, state }
    }

    pub fn state(&self) -> ItemState {
        self.state
    }

    pub fn take(&mut self) -> f32 {
        self.state = ItemState::Used;
        self.sample
    }
}

impl Default for Sample {
    fn default() -> Self {
        Self {
            sample: Default::default(),
            state: ItemState::Empty,
        }
    }
}

pub struct SampleBuffer<const N: usize> {
    r_idx: RwLock<usize>,
    w_idx: RwLock<usize>,
    buffer: RwLock<[Sample; N]>,
}

impl<const N: usize> SampleBuffer<N> {
    fn read_r_idx(&self) -> Option<usize> {
        if let Ok(index_guard) = self.r_idx.read() {
            return Some(*index_guard);
        }
        None
    }

    fn write_r_idx(&self, index: usize) {
        if let Ok(mut index_guard) = self.r_idx.write() {
            *index_guard = index;
        }
    }

    fn read_w_idx(&self) -> Option<usize> {
        if let Ok(index_guard) = self.w_idx.read() {
            return Some(*index_guard);
        }
        None
    }

    fn write_w_idx(&self, index: usize) {
        if let Ok(mut index_guard) = self.w_idx.write() {
            *index_guard = index;
        }
    }
}

impl<const N: usize> SampleBuffer<N> {
    pub fn new() -> Arc<Self> {
        let r_idx: RwLock<usize> = RwLock::new(0);
        let w_idx = RwLock::new(0);
        let buffer: RwLock<[Sample; N]> = RwLock::new([Sample::default(); N]);
        Arc::new(Self {
            r_idx,
            w_idx,
            buffer,
        })
    }

    pub fn add(self: &Arc<Self>, sample: f32) {
        if let Ok(mut buffer_guard) = self.buffer.write() {
            if let Some(w_idx) = self.read_w_idx() {
                if w_idx >= N {
                    buffer_guard[0] = Sample::new(sample);
                    self.write_r_idx(0);
                    self.write_w_idx(1);
                    return;
                }

                buffer_guard[w_idx] = Sample::new(sample);
                self.write_w_idx(w_idx + 1);
            }
        }
    }

    pub fn take(self: &Arc<Self>) -> Option<f32> {
        if let Ok(buffer_guard) = self.buffer.read() {
            if let Some(index) = self.read_r_idx() {
                if index < buffer_guard.len() {
                    let sample: &Sample = &buffer_guard[index as usize];
                    match sample.state() {
                        ItemState::Available => {
                            return Some(sample.take());
                        }
                        ItemState::Used => {
                            self.write_r_idx(index + 1);
                        }
                        ItemState::Empty => {}
                    }
                }
            }
        }
        None
    }
}
