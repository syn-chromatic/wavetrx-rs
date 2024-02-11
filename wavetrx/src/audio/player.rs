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

use super::types::AudioSpec;
use super::types::NormSamples;
use super::types::SampleBuffer;

pub struct OutputPlayer {
    device: Device,
    config: StreamConfig,
    spec: Arc<AudioSpec>,
    buffer: Arc<SampleBuffer>,
    stream: Option<Stream>,
}

impl OutputPlayer {
    pub fn new(device: Device, config: StreamConfig, spec: AudioSpec) -> Self {
        let buffer: Arc<SampleBuffer> = SampleBuffer::new();
        let spec: Arc<AudioSpec> = Arc::new(spec);
        let stream: Option<Stream> = None;
        Self {
            device,
            config,
            spec,
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
        self.buffer.add_sample(sample);
    }

    pub fn add_samples(&self, samples: NormSamples) {
        self.buffer.add_samples(samples);
    }

    pub fn wait(&self) {
        while !self.buffer.buffer_empty() {}
    }
}

impl OutputPlayer {
    fn append_mono(data: &mut [f32], buffer: &Arc<SampleBuffer>) {
        let mut count: usize = 0;
        while count < data.len() {
            if let Some(sample) = buffer.take() {
                data[count] = sample;
                data[count + 1] = sample;
                count += 2;
                continue;
            }
            break;
        }
    }

    fn append_stereo(data: &mut [f32], buffer: &Arc<SampleBuffer>) {
        let mut count: usize = 0;
        while count < data.len() {
            if let Some(sample) = buffer.take() {
                data[count] = sample;
                count += 1;
                continue;
            }
            break;
        }
    }

    fn data_callback(
        buffer: Arc<SampleBuffer>,
        spec: Arc<AudioSpec>,
    ) -> impl FnMut(&mut [f32], &OutputCallbackInfo) {
        let callback = move |data: &mut [f32], _: &OutputCallbackInfo| {
            // Sometimes the data buffer remains filled from previous frame
            if data.iter().any(|&value| value > 0.0) {
                for data in data.iter_mut() {
                    *data = 0.0;
                }
            }

            if !buffer.buffer_empty() {
                match spec.channels() {
                    1 => Self::append_mono(data, &buffer),
                    2 => Self::append_stereo(data, &buffer),
                    _ => {}
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
            Self::data_callback(self.buffer.clone(), self.spec.clone()),
            Self::error_callback,
            None,
        )?;
        Ok(stream)
    }
}
