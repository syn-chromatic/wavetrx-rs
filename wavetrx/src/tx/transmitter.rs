use std::fs::File;
use std::io::BufWriter;

use hound;
use hound::WavSpec;
use hound::WavWriter;

use super::tone::ToneGenerator;
use crate::audio::types::AudioSpec;
use crate::protocol::profile::ProtocolProfile;

pub struct Transmitter {
    profile: ProtocolProfile,
    spec: AudioSpec,
}

impl Transmitter {
    pub fn new(profile: ProtocolProfile, spec: &AudioSpec) -> Self {
        let spec: AudioSpec = spec.clone();
        Transmitter { profile, spec }
    }

    pub fn create(&self, data: &[u8]) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
        let spec: WavSpec = self.spec.into();
        let mut tone: ToneGenerator = ToneGenerator::new(spec)?;
        let fade_ratio: f32 = 0.1;

        self.append_silence(&mut tone)?;
        self.append_start(&mut tone, fade_ratio)?;
        self.append_next(&mut tone, fade_ratio)?;

        for &byte in data.iter() {
            self.append_byte(&mut tone, byte, fade_ratio)?;
        }

        self.append_end(&mut tone, fade_ratio)?;
        self.append_next(&mut tone, fade_ratio)?;
        self.append_silence(&mut tone)?;
        Ok(tone.samples())
    }

    pub fn create_file(
        &self,
        filename: &str,
        data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let spec: WavSpec = self.spec.into();
        let samples: Vec<i32> = self.create(data)?;

        let mut writer: WavWriter<BufWriter<File>> = WavWriter::create(filename, spec)?;
        for sample in samples {
            writer.write_sample(sample)?;
        }

        Ok(())
    }
}

impl Transmitter {
    fn append_byte(
        &self,
        tone: &mut ToneGenerator,
        byte: u8,
        fade_ratio: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for i in (0..8).rev() {
            let bit: bool = (byte & (1 << i)) != 0;
            self.append_bit(tone, bit, fade_ratio)?;
            self.append_next(tone, fade_ratio)?;
        }
        Ok(())
    }

    fn append_start(
        &self,
        tone: &mut ToneGenerator,
        fade_ratio: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tone_duration: usize = self.profile.pulses.tone.as_micros::<usize>();
        let gap_duration: usize = self.profile.pulses.gap.as_micros::<usize>();
        let frequency: f32 = self.profile.markers.start.hz();

        tone.append_sine_faded_tone(frequency, tone_duration, fade_ratio)?;
        tone.append_tone(0.0, gap_duration)?;
        Ok(())
    }

    fn append_end(
        &self,
        tone: &mut ToneGenerator,
        fade_ratio: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tone_duration: usize = self.profile.pulses.tone.as_micros::<usize>();
        let gap_duration: usize = self.profile.pulses.gap.as_micros::<usize>();
        let frequency: f32 = self.profile.markers.end.hz();

        tone.append_sine_faded_tone(frequency, tone_duration, fade_ratio)?;
        tone.append_tone(0.0, gap_duration)?;
        Ok(())
    }

    fn append_next(
        &self,
        tone: &mut ToneGenerator,
        fade_ratio: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tone_duration: usize = self.profile.pulses.tone.as_micros::<usize>();
        let gap_duration: usize = self.profile.pulses.gap.as_micros::<usize>();
        let frequency: f32 = self.profile.markers.next.hz();

        tone.append_sine_faded_tone(frequency, tone_duration, fade_ratio)?;
        tone.append_tone(0.0, gap_duration)?;
        Ok(())
    }

    fn append_silence(&self, tone: &mut ToneGenerator) -> Result<(), Box<dyn std::error::Error>> {
        let gap_duration: usize = self.profile.pulses.gap.as_micros::<usize>();
        let gap_duration = gap_duration * 4;
        tone.append_tone(0.0, gap_duration)?;
        Ok(())
    }

    fn append_bit(
        &self,
        tone: &mut ToneGenerator,
        bit: bool,
        fade_ratio: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let frequency: f32 = self.profile.bits.from_boolean(bit).hz();
        let tone_duration: usize = self.profile.pulses.tone.as_micros::<usize>();
        let gap_duration: usize = self.profile.pulses.gap.as_micros::<usize>();

        tone.append_sine_faded_tone(frequency, tone_duration, fade_ratio)?;
        tone.append_tone(0.0, gap_duration)?;
        Ok(())
    }
}
