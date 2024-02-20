use std::fs::File;
use std::io::BufWriter;
use std::slice::Iter;

use hound;
use hound::WavSpec;
use hound::WavWriter;

use super::tone::ToneGenerator;
use crate::audio::types::AudioSpec;
use crate::protocol::profile::Profile;

pub struct Transmitter {
    profile: Profile,
    spec: AudioSpec,
}

impl Transmitter {
    pub fn new(profile: &Profile, spec: &AudioSpec) -> Self {
        let profile: Profile = *profile;
        let spec: AudioSpec = spec.clone();

        Transmitter { profile, spec }
    }

    pub fn create(&self, data: &[u8]) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let mut tone: ToneGenerator = ToneGenerator::new(&self.spec)?;
        let fade: f32 = 0.1;

        self.append_silence(&mut tone)?;
        self.append_start(&mut tone, fade)?;
        self.append_next(&mut tone, fade)?;

        for &byte in data.iter() {
            self.append_byte(&mut tone, byte, fade)?;
        }

        self.append_end(&mut tone, fade)?;
        self.append_next(&mut tone, fade)?;
        self.append_silence(&mut tone)?;
        Ok(tone.samples())
    }

    pub fn create_file(
        &self,
        filename: &str,
        data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let samples: Vec<f32> = self.create(data)?;

        let spec: WavSpec = self.spec.into();
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
        fade: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for i in (0..8).rev() {
            let bit: bool = (byte & (1 << i)) != 0;
            self.append_bit(tone, bit, fade)?;
            self.append_next(tone, fade)?;
        }
        Ok(())
    }

    fn append_start(
        &self,
        tone: &mut ToneGenerator,
        fade: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tone_duration: usize = self.profile.pulses.tone.as_micros::<usize>();
        let gap_duration: usize = self.profile.pulses.gap.as_micros::<usize>();
        let frequency: f32 = self.profile.markers.start.hz();

        tone.append_sine_faded_tone(frequency, tone_duration, fade)?;
        tone.append_tone(0.0, gap_duration)?;
        Ok(())
    }

    fn append_end(
        &self,
        tone: &mut ToneGenerator,
        fade: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tone_duration: usize = self.profile.pulses.tone.as_micros::<usize>();
        let gap_duration: usize = self.profile.pulses.gap.as_micros::<usize>();
        let frequency: f32 = self.profile.markers.end.hz();

        tone.append_sine_faded_tone(frequency, tone_duration, fade)?;
        tone.append_tone(0.0, gap_duration)?;
        Ok(())
    }

    fn append_next(
        &self,
        tone: &mut ToneGenerator,
        fade: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tone_duration: usize = self.profile.pulses.tone.as_micros::<usize>();
        let gap_duration: usize = self.profile.pulses.gap.as_micros::<usize>();
        let frequency: f32 = self.profile.markers.next.hz();

        tone.append_sine_faded_tone(frequency, tone_duration, fade)?;
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
        fade: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let frequency: f32 = self.profile.bits.from_boolean(bit).hz();
        let tone_duration: usize = self.profile.pulses.tone.as_micros::<usize>();
        let gap_duration: usize = self.profile.pulses.gap.as_micros::<usize>();

        tone.append_sine_faded_tone(frequency, tone_duration, fade)?;
        tone.append_tone(0.0, gap_duration)?;
        Ok(())
    }
}

enum StreamTxStage {
    Start,
    Data,
    End,
}

pub struct StreamTransmitter<'a, const N: usize> {
    tx: Transmitter,
    tone: ToneGenerator,
    stage: StreamTxStage,
    data: Iter<'a, u8>,
    fade: f32,
    close: bool,
}

impl<'a, const N: usize> StreamTransmitter<'a, N> {
    pub fn new(profile: &Profile, spec: &AudioSpec, data: &'a [u8]) -> Self {
        let tx: Transmitter = Transmitter::new(profile, spec);
        let tone: ToneGenerator = ToneGenerator::new(spec).unwrap();
        let stage: StreamTxStage = StreamTxStage::Start;
        let data: Iter<'a, u8> = data.iter();
        let fade: f32 = 0.0;
        let close: bool = false;

        Self {
            tx,
            tone,
            stage,
            data,
            fade,
            close,
        }
    }

    pub fn set_fade(&mut self, fade: f32) {
        self.fade = fade;
    }
}

impl<'a, const N: usize> Iterator for StreamTransmitter<'a, N> {
    type Item = Vec<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.close {
            return None;
        }

        for _ in 0..N {
            match self.stage {
                StreamTxStage::Start => {
                    self.tx.append_silence(&mut self.tone).unwrap();
                    self.tx.append_start(&mut self.tone, self.fade).unwrap();
                    self.tx.append_next(&mut self.tone, self.fade).unwrap();
                    self.stage = StreamTxStage::Data;
                }
                StreamTxStage::Data => {
                    if let Some(&byte) = self.data.next() {
                        self.tx
                            .append_byte(&mut self.tone, byte, self.fade)
                            .unwrap();
                    } else {
                        self.stage = StreamTxStage::End;
                    }
                }
                StreamTxStage::End => {
                    self.tx.append_end(&mut self.tone, self.fade).unwrap();
                    self.tx.append_next(&mut self.tone, self.fade).unwrap();
                    self.tx.append_silence(&mut self.tone).unwrap();
                    self.close = true;
                    break;
                }
            };
        }

        Some(self.tone.take_samples())
    }
}
