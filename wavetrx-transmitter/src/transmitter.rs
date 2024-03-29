use std::io;
use std::io::Write;
use std::time::Duration;

use cpal::Device;
use cpal::Host;
use cpal::SupportedStreamConfig;

use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;

use wavetrx::audio::player::OutputPlayer;
use wavetrx::audio::types::AudioSpec;
use wavetrx::audio::types::NormSamples;
use wavetrx::audio::types::SampleEncoding;

use wavetrx::protocol::profile::Profile;
use wavetrx::protocol::tx::StreamTransmitter;
use wavetrx::protocol::tx::Transmitter;

use wavetrx::utils::get_fast_profile;

fn input(prompt: &str) -> String {
    let mut input: String = String::new();
    print!("{}", prompt);

    io::stdout().flush().unwrap();

    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    input.trim().to_string()
}

fn transmit_string(
    string: &str,
    transmitter: &Transmitter,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let data: &[u8] = string.as_bytes();
    let result: Result<Vec<f32>, Box<dyn std::error::Error>> = transmitter.create(data);

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

pub fn display_profile(profile: &Profile, spec: &AudioSpec) {
    let min_freq_sep: f32 = profile.min_frequency_separation(spec);

    println!("{:?}", profile);
    println!("Min Freq Sep: {:?} Hz", min_freq_sep);
    println!();
}

pub fn transmitter_player() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[Transmitter]\n");
    let (device, config): (Device, SupportedStreamConfig) = get_default_output_device()?;

    let spec: AudioSpec = get_mono_audio_spec_f32(&config);
    let profile: Profile = get_fast_profile();
    display_profile(&profile, &spec);

    let transmitter: Transmitter = Transmitter::new(&profile, &spec);

    let mut player: OutputPlayer = OutputPlayer::new(device, config.into(), spec);
    player.play()?;

    loop {
        let string: String = input("Input: ");
        if let Ok(samples) = transmit_string(&string, &transmitter) {
            let samples: NormSamples = NormSamples::from_slice(&samples);
            let timestamp: Duration = spec.sample_timestamp(samples.0.len());
            println!("Length: {:?}s", timestamp.as_millis() as f32 / 1e3);
            player.add_samples(samples);

            player.wait();
            println!();
        }
    }
}

pub fn stream_transmitter_player() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[Transmitter]\n");
    let (device, config): (Device, SupportedStreamConfig) = get_default_output_device()?;

    let spec: AudioSpec = get_mono_audio_spec_f32(&config);
    let profile: Profile = get_fast_profile();
    display_profile(&profile, &spec);

    let mut player: OutputPlayer = OutputPlayer::new(device, config.into(), spec);
    player.play()?;

    const TX_BUFFER: usize = 256;

    loop {
        let string: String = input("Input: ");
        let data: &[u8] = string.as_bytes();
        let stream_transmitter: StreamTransmitter<'_, TX_BUFFER> =
            StreamTransmitter::new(&profile, &spec, data);

        for stream_samples in stream_transmitter {
            let stream_samples: NormSamples = NormSamples::from_vec(stream_samples);
            player.add_samples(stream_samples);
            player.wait_until(4096);
        }
    }
}
