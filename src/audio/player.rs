use std::error;
use std::sync::Arc;

use cpal::traits::DeviceTrait;
use cpal::traits::StreamTrait;
use cpal::BuildStreamError;
use cpal::Device;
use cpal::OutputCallbackInfo;
use cpal::Stream;
use cpal::StreamConfig;
use cpal::StreamError;

use super::types::SampleBuffer;

pub struct Player {
    device: Device,
    config: StreamConfig,
    buffer: Arc<SampleBuffer<256>>,
    stream: Option<Stream>,
}

impl Player {
    pub fn new(device: Device, config: StreamConfig) -> Self {
        let buffer: Arc<SampleBuffer<256>> = SampleBuffer::new();
        let stream: Option<Stream> = None;
        Self {
            device,
            config,
            buffer,
            stream,
        }
    }

    pub fn play(&mut self) -> Result<(), Box<dyn error::Error>> {
        let stream: Stream = self.build_output_stream()?;
        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn add_sample(&self, sample: f32) {
        self.buffer.add(sample);
    }
}

impl Player {
    fn data_callback(buffer: Arc<SampleBuffer<256>>) -> impl FnMut(&mut [f32], &OutputCallbackInfo) {
        let callback = move |data: &mut [f32], info: &OutputCallbackInfo| {
            let mut count: usize = 0;
            while count < data.len() {
                if let Some(sample) = buffer.take() {
                    data[count] = sample;
                    count += 1;
                }
            }
        };
        callback
    }

    fn error_callback(err: StreamError) {
        println!("Error: {:?}", err);
    }

    fn build_output_stream(&mut self) -> Result<Stream, BuildStreamError> {
        let stream: Stream = self.device.build_output_stream(
            &self.config,
            Self::data_callback(self.buffer.clone()),
            Self::error_callback,
            None,
        )?;
        Ok(stream)
    }
}
