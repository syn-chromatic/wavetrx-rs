use std::thread::sleep;
use std::time::Duration;

use cpal::Device;
use cpal::Host;
use cpal::SampleFormat;
use cpal::SupportedStreamConfig;

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;

use wavetrx::audio::recorder::InputRecorder;

use wavetrx::audio::types::AudioSpec;
use wavetrx::audio::types::NormSamples;
use wavetrx::audio::types::SampleEncoding;

use wavetrx::protocol::profile::ProtocolProfile;
use wavetrx::rx::receiver::LiveReceiver;

use wavetrx::get_profile;

pub fn print_config(config: &SupportedStreamConfig) {
    let channels: u16 = config.channels();
    let sample_rate: u32 = config.sample_rate().0;
    let sample_format: SampleFormat = config.sample_format();
    let bits_per_sample: u16 = (sample_format.sample_size() * 8) as u16;
    println!("Channels: {}", channels);
    println!("Sample Rate: {}", sample_rate);
    println!("Sample Size: {}", sample_format.sample_size());
    println!("Bits Per Sample: {}", bits_per_sample);
}

pub fn get_default_output_device(
) -> Result<(Device, SupportedStreamConfig), Box<dyn std::error::Error>> {
    let host: Host = cpal::default_host();
    let device: Device = host
        .default_output_device()
        .ok_or("No output device available")?;
    let config: SupportedStreamConfig = device.default_output_config()?;

    Ok((device, config))
}

pub fn get_mono_audio_spec_i32(config: &SupportedStreamConfig) -> AudioSpec {
    let sample_rate: u32 = config.sample_rate().0;
    let sample_format: SampleFormat = config.sample_format();
    let bps: u16 = (sample_format.sample_size() * 8) as u16;
    let channels: u16 = 1;
    let encoding: SampleEncoding = SampleEncoding::I32;
    let spec: AudioSpec = AudioSpec::new(sample_rate, bps, channels, encoding);
    spec
}
pub fn live_output_receiver() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[Live Receiver]\n");
    let (device, config): (Device, SupportedStreamConfig) = get_default_output_device()?;

    println!("Output device: {}", device.name()?);
    print_config(&config);

    let spec: AudioSpec = get_mono_audio_spec_i32(&config);

    let profile: ProtocolProfile = get_profile();
    let mut live_receiver: LiveReceiver = LiveReceiver::new(profile, spec);

    let mut recorder: InputRecorder = InputRecorder::new(device, config.into());
    recorder.record()?;

    println!("\n[Messages]");

    loop {
        if let Some(samples) = recorder.take_frame() {
            let mut sc_samples: NormSamples = NormSamples::new();
            for (idx, sample) in samples.0.iter().enumerate() {
                if idx % 2 == 0 {
                    sc_samples.0.push(*sample);
                }
            }

            live_receiver.add_samples(&mut sc_samples);
            continue;
        }
        sleep(Duration::from_millis(50));
    }
}
