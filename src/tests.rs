use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavReader, WavSpec};

use crate::protocol::ProtocolProfile;
use crate::rx::receiver::{save_normalized_name, LiveReceiver, Receiver};
use crate::rx::spectrum::Normalizer;
use crate::tx::transmitter::Transmitter;
use crate::utils::bits_to_string;
use crate::utils::get_bit_depth_magnitudes;

use crate::consts::{AUDIO_BPS, AUDIO_SR, MIN_FREQ_SEP};
use crate::get_profile;

#[test]
fn test_transmitter() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    let filename: &str = "transmitted_audio.wav";
    let string: &str = "Hello there!";
    let data: &[u8] = string.as_bytes();

    println!("Data: {:?}", data);

    let profile: ProtocolProfile = get_profile();
    let sample_rate: usize = AUDIO_SR;
    let bit_depth: usize = AUDIO_BPS;

    let transmitter: Transmitter = Transmitter::new(profile, sample_rate, bit_depth);
    let result: Result<(), hound::Error> = transmitter.create_file(filename, data);

    if let Err(err) = result {
        println!("Error: Failed to generate data: {:?}", err);
        return;
    }

    println!("Generated {} bytes", data.len());
}

#[test]
fn test_receiver() {
    println!("MIN FREQUENCY SEPARATION: {} hz", MIN_FREQ_SEP);
    // let filename: &str = "transmitted_audio.wav";
    let filename: &str = "test8.wav";
    // let filename: &str = "maximized_audio.wav";

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
    let filename: &str = "live_transmit_test.wav";

    let (mut samples, spec) = read_file(filename);
    let profile: ProtocolProfile = get_profile();
    let mut live_receiver: LiveReceiver = LiveReceiver::new(profile, spec);
    // let sample_size = live_receiver.get_sample_size();
    let sample_size: usize = 44;

    let mut idx = 0;
    while idx + sample_size < samples.len() {
        let timestamp = idx as f32 / spec.sample_rate as f32;
        println!("Timestamp: {:.3}", timestamp);
        let en_index: usize = idx + sample_size;
        let samples_chunk: &mut [f32] = &mut samples[idx..en_index];
        live_receiver.append_audio_samples(samples_chunk);
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

    let spec = WavSpec {
        channels: 1,
        sample_rate: sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Int,
    };

    let profile: ProtocolProfile = get_profile();
    let mut live_receiver: LiveReceiver = LiveReceiver::new(profile, spec);
    let recorded_samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded_samples_arc: Arc<Mutex<Vec<f32>>> = recorded_samples.clone();
    let (p_magnitude, n_magnitude) = get_bit_depth_magnitudes(bits_per_sample as usize);

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut samples: Vec<f32> = Vec::new();
            for (idx, sample) in data.iter().enumerate() {
                if idx % 2 == 0 {
                    samples.push(*sample);
                }
            }

            live_receiver.append_audio_samples(&mut samples);
            // recorded_samples_arc.lock().unwrap().append(&mut samples);
        },
        move |err| {
            eprintln!("An error occurred on stream: {}", err);
        },
        None,
    )?;

    stream.play()?;
    std::thread::sleep(std::time::Duration::from_secs(180));

    // if let Ok(recorded) = recorded_samples.lock().as_deref() {
    //     save_normalized_name("recorded.wav", recorded, &spec);
    // }

    Ok(())
}

fn read_file(filename: &str) -> (Vec<f32>, WavSpec) {
    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let samples: Vec<f32> = samples.iter().map(|&sample| sample as f32).collect();
    let spec: WavSpec = reader.spec();
    (samples, spec)
}
