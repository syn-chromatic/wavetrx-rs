use std::fs::File;
use std::io::BufReader;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use hound::{WavReader, WavSpec};

use crate::audio::player::OutputPlayer;
use crate::audio::recorder::InputRecorder;

use crate::audio::types::AudioSpec;
use crate::audio::types::SampleEncoding;

use crate::protocol::profile::ProtocolProfile;
use crate::protocol::utils::bits_to_string;
use crate::rx::receiver::{save_normalized_name, LiveReceiver, Receiver};
use crate::rx::spectrum::Normalizer;
use crate::tx::transmitter::Transmitter;

use crate::audio::utils::save_audio;
use crate::consts::{AUDIO_BPS, AUDIO_SR, MIN_FREQ_SEP};
use crate::get_profile;

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
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    let filename: &str = "transmitted_audio.wav";
    let string: String = "Test String".repeat(100);
    let data: &[u8] = string.as_bytes();

    println!("Data: {:?}", data);

    let profile: ProtocolProfile = get_profile();
    let sample_rate: usize = AUDIO_SR;
    let bit_depth: usize = AUDIO_BPS;

    let transmitter: Transmitter = Transmitter::new(profile, sample_rate, bit_depth);
    let result: Result<(), Box<dyn std::error::Error>> = transmitter.create_file(filename, data);

    if let Err(err) = result {
        println!("Error: Failed to generate data: {:?}", err);
        return;
    }

    println!("Generated {} bytes", data.len());
}

#[test]
fn test_receiver() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    let filename: &str = "transmitted_audio.wav";

    let profile: ProtocolProfile = get_profile();
    let receiver: Receiver = Receiver::new(profile);

    let bits: Option<Vec<u8>> = receiver.from_file(filename);

    if let Some(bits) = bits {
        println!("{}", "-".repeat(20));
        println!();
        for bit in bits.iter() {
            print!("{}", bit);
        }
        println!();

        let string: String = bits_to_string(&bits);
        println!("Decoded: {}", string);
        println!();
        println!("{}", "-".repeat(20));
    }
}

#[test]
fn test_live_receiver() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    let filename: &str = "transmitted_audio.wav";

    let (mut samples, spec) = read_file(filename);
    let spec: AudioSpec = spec.into();
    let profile: ProtocolProfile = get_profile();

    let mut live_receiver: LiveReceiver = LiveReceiver::new(profile, spec);
    let sample_size = live_receiver.get_sample_size();
    let sample_size: usize = 44;

    let mut idx = 0;
    while idx + sample_size < samples.len() {
        let timestamp = idx as f32 / spec.sample_rate() as f32;
        // println!("Timestamp: {:.3}", timestamp);
        let en_index: usize = idx + sample_size;
        let samples_chunk: &mut [f32] = &mut samples[idx..en_index];
        live_receiver.append_sample(samples_chunk);
        idx += sample_size;
    }

    // live_receiver.save("live_receiver_output.wav");
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

    let profile: ProtocolProfile = get_profile();
    let mut live_receiver: LiveReceiver = LiveReceiver::new(profile, spec);
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

            live_receiver.append_sample(&mut samples);
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

    let profile: ProtocolProfile = get_profile();
    let mut live_receiver: LiveReceiver = LiveReceiver::new(profile, spec);

    let mut recorder: InputRecorder = InputRecorder::new(device, config.into());
    recorder.record()?;

    println!("Live Receiver");

    let mut frames: Vec<f32> = Vec::new();

    loop {
        if let Some(frame) = recorder.take_frame() {
            // println!("Samples: {}", sample.len());
            let mut sc_frame: Vec<f32> = Vec::new();
            for (idx, sample) in frame.iter().enumerate() {
                if idx % 2 == 0 {
                    sc_frame.push(*sample);
                }
            }

            frames.extend(frame);

            live_receiver.append_sample(&mut sc_frame);
        }

        if frames.len() >= 1_000_000 {
            break;
        }
    }

    save_normalized_name("record_audio_test.wav", &frames, &spec);
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

    let profile: ProtocolProfile = get_profile();
    let mut live_receiver: LiveReceiver = LiveReceiver::new(profile, spec);

    let mut recorder: InputRecorder = InputRecorder::new(device, config.into());
    recorder.record()?;

    println!("\n[Live Receiver]");

    // let mut samples: Vec<f32> = Vec::new();

    loop {
        if let Some(frame) = recorder.take_frame() {
            // println!("Samples: {}", sample.len());
            let mut sc_frame: Vec<f32> = Vec::new();
            for (idx, sample) in frame.iter().enumerate() {
                if idx % 2 == 0 {
                    sc_frame.push(*sample);
                }
            }

            // samples.extend(frame);

            live_receiver.append_sample(&mut sc_frame);
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

    println!("ALL SAMPLES: {}", samples.len());
    for sample in samples {
        player.add_sample(sample);
    }

    println!("Done!");
    std::thread::sleep(std::time::Duration::from_secs(180));

    Ok(())
}

fn transmit_string(string: &str) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
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

// #[test]
pub fn test_transmitter_player() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device available")?;
    let config = device.default_output_config()?;

    let sample_rate = config.sample_rate().0;
    let spec: AudioSpec = AudioSpec::new(sample_rate, 4, 1, SampleEncoding::F32);

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

    Ok(())
}

fn read_file(filename: &str) -> (Vec<f32>, WavSpec) {
    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let samples: Vec<f32> = samples.iter().map(|&sample| sample as f32).collect();
    let spec: WavSpec = reader.spec();
    (samples, spec)
}

fn read_wav_file(file_path: &str) -> (Vec<f32>, WavSpec) {
    let mut reader: WavReader<BufReader<File>> = hound::WavReader::open(file_path).unwrap();
    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let samples: Vec<f32> = samples
        .iter()
        .map(|&s| (s as f32) / (i32::MAX as f32))
        .collect();
    let spec: WavSpec = reader.spec();
    (samples, spec)
}
