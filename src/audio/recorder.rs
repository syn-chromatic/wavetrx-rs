use std::error;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use cpal::traits::DeviceTrait;
use cpal::traits::StreamTrait;
use cpal::BuildStreamError;
use cpal::Device;
use cpal::InputCallbackInfo;
use cpal::Stream;
use cpal::StreamConfig;
use cpal::StreamError;

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

pub struct Recorder {
    device: Device,
    config: StreamConfig,
    buffer: Arc<Buffer>,
    stream: Option<Stream>,
}

impl Recorder {
    pub fn new(device: Device, config: StreamConfig) -> Self {
        let buffer: Arc<Buffer> = Buffer::new();
        let stream: Option<Stream> = None;
        Self {
            device,
            config,
            buffer,
            stream,
        }
    }

    pub fn record(&mut self) -> Result<(), Box<dyn error::Error>> {
        let stream: Stream = self.build_input_stream()?;
        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn take_sample(&mut self) -> Option<Vec<f32>> {
        self.buffer.take()
    }
}

impl Recorder {
    fn data_callback(buffer: Arc<Buffer>) -> impl Fn(&[f32], &InputCallbackInfo) {
        let callback = move |data: &[f32], info: &InputCallbackInfo| {
            let data: Vec<f32> = data.to_vec();
            buffer.add(data);
        };
        callback
    }

    fn error_callback(err: StreamError) {
        println!("Error: {:?}", err);
    }

    fn build_input_stream(&mut self) -> Result<Stream, BuildStreamError> {
        let stream: Stream = self.device.build_input_stream(
            &self.config,
            Self::data_callback(self.buffer.clone()),
            Self::error_callback,
            None,
        )?;
        Ok(stream)
    }
}
