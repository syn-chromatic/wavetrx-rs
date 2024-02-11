use std::error;
use std::sync::Arc;

use cpal::traits::DeviceTrait;
use cpal::traits::StreamTrait;
use cpal::BuildStreamError;
use cpal::Device;
use cpal::InputCallbackInfo;
use cpal::Stream;
use cpal::StreamConfig;
use cpal::StreamError;

use super::types::FrameBuffer;
use super::types::NormSamples;

pub struct InputRecorder {
    device: Device,
    config: StreamConfig,
    buffer: Arc<FrameBuffer>,
    stream: Option<Stream>,
}

impl InputRecorder {
    pub fn new(device: Device, config: StreamConfig) -> Self {
        let buffer: Arc<FrameBuffer> = FrameBuffer::new();
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

    pub fn take_frame(&mut self) -> Option<NormSamples> {
        self.buffer.take()
    }
}

impl InputRecorder {
    fn data_callback(buffer: Arc<FrameBuffer>) -> impl Fn(&[f32], &InputCallbackInfo) {
        let callback = move |data: &[f32], _: &InputCallbackInfo| {
            let frame: NormSamples = NormSamples::from_norm(data);
            buffer.add_frame(frame);
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
