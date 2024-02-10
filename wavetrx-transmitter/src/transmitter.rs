use std::io::{self, Write};

use cpal::Device;
use cpal::Host;
use cpal::SupportedStreamConfig;

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;

use wavetrx::audio::player::OutputPlayer;

use wavetrx::audio::types::AudioSpec;
use wavetrx::audio::types::SampleEncoding;

use wavetrx::protocol::profile::ProtocolProfile;
use wavetrx::tx::transmitter::Transmitter;

use wavetrx::consts::AUDIO_BPS;
use wavetrx::consts::AUDIO_SR;
use wavetrx::get_profile;

fn input(prompt: &str) -> String {
    let mut input: String = String::new();
    print!("{}", prompt);

    io::stdout().flush().unwrap();

    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    input.trim().to_string()
}

fn transmit_string(string: &str) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    let data: &[u8] = string.as_bytes();
    println!("Data: {:?}", data);

    let profile: ProtocolProfile = get_profile();
    let sample_rate: usize = AUDIO_SR;
    let bit_depth: usize = AUDIO_BPS;

    let transmitter: Transmitter = Transmitter::new(profile, sample_rate, bit_depth);
    let result: Result<Vec<i32>, Box<dyn std::error::Error>> = transmitter.create(data);

    if let Err(err) = result {
        panic!("Error: Failed to generate data: {:?}", err);
    }

    println!("Generated {} bytes", data.len());
    result
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

pub fn get_mono_audio_spec_f32(config: &SupportedStreamConfig) -> AudioSpec {
    let sample_rate: u32 = config.sample_rate().0;
    let sample_format: cpal::SampleFormat = config.sample_format();
    let bps: u16 = (sample_format.sample_size() * 8) as u16;
    let channels: u16 = 1;
    let encoding: SampleEncoding = SampleEncoding::F32;
    let spec: AudioSpec = AudioSpec::new(sample_rate, bps, channels, encoding);
    spec
}
pub fn transmitter_player() -> Result<(), Box<dyn std::error::Error>> {
    let (device, config): (Device, SupportedStreamConfig) = get_default_output_device()?;

    let spec: AudioSpec = get_mono_audio_spec_f32(&config);
    let mut player: OutputPlayer = OutputPlayer::new(device, config.into(), spec);
    player.play()?;

    println!("[Transmitter]");

    loop {
        let string: String = input("Input: ");
        if let Ok(samples) = transmit_string(&string) {
            for sample in samples {
                let sample = (sample as f32) / (i32::MAX as f32);
                player.add_sample(sample);
            }

            player.wait();
            println!();
        }
    }
}
