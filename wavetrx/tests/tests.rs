use std::fs::File;
use std::io::BufReader;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use hound::{WavReader, WavSpec};

use wavetrx::audio::player::OutputPlayer;
use wavetrx::audio::recorder::InputRecorder;

use wavetrx::audio::types::AudioSpec;
use wavetrx::audio::types::SampleEncoding;

use wavetrx::audio::spectrum::Normalizer;
use wavetrx::audio::types::NormSamples;
use wavetrx::protocol::profile::Profile;
use wavetrx::protocol::rx::Receiver;

use wavetrx::protocol::tx::Transmitter;
use wavetrx::utils::bits_to_string;
use wavetrx::utils::read_wav_file;

use wavetrx::utils::get_default_profile;

fn input(prompt: &str) -> String {
    let mut input: String = String::new();
    print!("{}", prompt);

    io::stdout().flush().unwrap(); // Ensure the prompt is displayed immediately

    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    input.trim().to_string() // Trimming to remove any trailing newline characters
}

#[test]
fn test_transmitter() {
    let filename: &str = "transmitted_audio.wav";
    let string: String = "Test String".repeat(100);
    let data: &[u8] = string.as_bytes();

    println!("Data: {:?}", data);

    let profile: Profile = get_default_profile();

    let spec: AudioSpec = AudioSpec::new(48_000, 32, 1, SampleEncoding::F32);
    let transmitter: Transmitter = Transmitter::new(&profile, &spec);
    let result: Result<(), Box<dyn std::error::Error>> = transmitter.create_file(filename, data);

    if let Err(err) = result {
        println!("Error: Failed to generate data: {:?}", err);
        return;
    }

    println!("Generated {} bytes", data.len());
}

#[test]
fn test_live_recording_receiver() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No input device available")?;
    let config = device.default_input_config()?;

    println!("Default input device: {}", device.name()?);
    println!("Default input format: {:?}", config);

    let channels = config.channels();
    let sample_rate = config.sample_rate().0;
    let sample_format = config.sample_format();
    let bits_per_sample = (sample_format.sample_size() * 8) as u16;
    println!("Channels: {}", channels);
    println!("Sample Rate: {}", sample_rate);
    println!("Sample Size: {}", sample_format.sample_size());
    println!("Bits Per Sample: {}", bits_per_sample);

    let spec: AudioSpec = AudioSpec::new(sample_rate, 32, 1, SampleEncoding::I32);

    let profile: Profile = get_default_profile();
    let mut receiver: Receiver = Receiver::new(profile, spec);
    let recorded_samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded_samples_arc: Arc<Mutex<Vec<f32>>> = recorded_samples.clone();

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // println!("Len Data: {}", data.len());
            let mut samples: Vec<f32> = Vec::new();
            for (idx, sample) in data.iter().enumerate() {
                if idx % 2 == 0 {
                    samples.push(*sample);
                }
            }

            let mut samples: NormSamples = NormSamples::from_slice(&samples);
            receiver.add_samples(&mut samples);
            receiver.analyze_buffer();
            // recorded_samples_arc.lock().unwrap().append(&mut samples);
        },
        move |err| {
            eprintln!("An error occurred on stream: {}", err);
        },
        None,
    )?;

    stream.play()?;
    println!("PLAYED");
    std::thread::sleep(std::time::Duration::from_secs(180));

    // if let Ok(recorded) = recorded_samples.lock().as_deref() {
    //     save_normalized_name("recorded.wav", recorded, &spec);
    // }

    Ok(())
}

#[test]
fn test_live_recording_receiver2() -> Result<(), Box<dyn std::error::Error>> {
    let host: cpal::Host = cpal::default_host();
    let device: cpal::Device = host
        .default_input_device()
        .ok_or("No input device available")?;
    let config = device.default_input_config()?;

    println!("Default input device: {}", device.name()?);
    println!("Default input format: {:?}", config);

    let channels: u16 = config.channels();
    let sample_rate: u32 = config.sample_rate().0;
    let sample_format = config.sample_format();
    let bits_per_sample: u16 = (sample_format.sample_size() * 8) as u16;
    println!("Channels: {}", channels);
    println!("Sample Rate: {}", sample_rate);
    println!("Sample Size: {}", sample_format.sample_size());
    println!("Bits Per Sample: {}", bits_per_sample);

    let spec: AudioSpec = AudioSpec::new(sample_rate, bits_per_sample, 1, SampleEncoding::I32);

    let profile: Profile = get_default_profile();
    let mut receiver: Receiver = Receiver::new(profile, spec);

    let mut recorder: InputRecorder = InputRecorder::new(device, config.into());
    recorder.record()?;

    println!("Live Receiver");

    let mut frames: Vec<f32> = Vec::new();

    loop {
        if let Some(samples) = recorder.take_frame() {
            // println!("Samples: {}", sample.len());
            let mut sc_samples: Vec<f32> = Vec::new();
            for (idx, sample) in samples.0.iter().enumerate() {
                if idx % 2 == 0 {
                    sc_samples.push(*sample);
                }
            }

            frames.extend(samples.0);

            let mut samples: NormSamples = NormSamples::from_slice(&sc_samples);
            receiver.add_samples(&mut samples);
            receiver.analyze_buffer();
        }

        if frames.len() >= 1_000_000 {
            break;
        }
    }

    let samples: NormSamples = NormSamples::from_slice(&frames);
    samples.save_file("record_audio_test.wav", &spec);
    println!("Done");

    // std::thread::sleep(std::time::Duration::from_secs(180));

    Ok(())
}

// #[test]
pub fn test_live_recording_receiver3() -> Result<(), Box<dyn std::error::Error>> {
    let host: cpal::Host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device available")?;
    let config = device.default_output_config()?;

    println!("Output device: {}", device.name()?);
    // println!("Default Output format: {:?}", config);

    let channels: u16 = config.channels();
    let sample_rate: u32 = config.sample_rate().0;
    let sample_format = config.sample_format();
    let bits_per_sample: u16 = (sample_format.sample_size() * 8) as u16;
    println!("Channels: {}", channels);
    println!("Sample Rate: {}", sample_rate);
    println!("Sample Size: {}", sample_format.sample_size());
    println!("Bits Per Sample: {}", bits_per_sample);

    let spec: AudioSpec = AudioSpec::new(sample_rate, bits_per_sample, 1, SampleEncoding::I32);

    let profile: Profile = get_default_profile();
    let mut receiver: Receiver = Receiver::new(profile, spec);

    let mut recorder: InputRecorder = InputRecorder::new(device, config.into());
    recorder.record()?;

    println!("\n[Live Receiver]");

    // let mut samples: Vec<f32> = Vec::new();

    loop {
        if let Some(samples) = recorder.take_frame() {
            // println!("Samples: {}", sample.len());
            let mut sc_samples: Vec<f32> = Vec::new();
            for (idx, sample) in samples.0.iter().enumerate() {
                if idx % 2 == 0 {
                    sc_samples.push(*sample);
                }
            }

            // samples.extend(samples.0);

            let mut samples: NormSamples = NormSamples::from_slice(&sc_samples);
            receiver.add_samples(&mut samples);
            receiver.analyze_buffer();
        }

        // if samples.len() >= 500_000 {
        //     break;
        // }
    }

    // let spec: AudioSpec = AudioSpec::new(sample_rate, bits_per_sample, 2, SampleEncoding::F32);
    // // save_normalized_name("record_audio_test.wav", &samples, &spec);
    // save_audio("record_audio_test.wav", &samples, &spec);
    // println!("Done");

    Ok(())
}

#[test]
fn test_player() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device available")?;
    let config = device.default_output_config()?;

    println!("Default output device: {}", device.name()?);
    println!("Default output format: {:?}", config);

    let channels: u16 = config.channels();
    let sample_rate: u32 = config.sample_rate().0;
    let sample_format = config.sample_format();
    let bits_per_sample: u16 = (sample_format.sample_size() * 8) as u16;
    println!("Channels: {}", channels);
    println!("Sample Rate: {}", sample_rate);
    println!("Sample Size: {}", sample_format.sample_size());
    println!("Bits Per Sample: {}", bits_per_sample);

    let filename: &str = "music.wav";
    let (samples, spec) = read_wav_file(filename);
    let spec: AudioSpec = spec.into();

    let mut player: OutputPlayer = OutputPlayer::new(device, config.into(), spec);
    player.play()?;

    println!("WAV Sample Rate: {}", spec.sample_rate());
    println!("WAV Channels: {}", spec.channels());

    println!("ALL SAMPLES: {}", samples.0.len());
    for sample in samples.0 {
        player.add_sample(sample);
    }

    println!("Done!");
    std::thread::sleep(std::time::Duration::from_secs(180));

    Ok(())
}
