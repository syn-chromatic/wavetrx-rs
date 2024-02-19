use std::path::Path;

use super::resolver::RxMagnitudes;
use super::resolver::RxOutput;
use super::resolver::RxResolver;

use crate::audio::spectrum::FourierMagnitude;
use crate::audio::spectrum::Normalizer;
use crate::audio::types::AudioSpec;
use crate::audio::types::NormSamples;

use crate::protocol::profile::Profile;
use crate::protocol::profile::SizedPulses;
use crate::utils::bits_to_string;
use crate::utils::read_wav_file;

use crate::consts::DB_THRESHOLD;

pub struct Receiver {
    profile: Profile,
    pulses: SizedPulses,
    spec: AudioSpec,
    bits: Vec<u8>,
    buffer: NormSamples,
    resolver: RxResolver,
    magnitude: FourierMagnitude,
    st_idx: Option<usize>,
}

impl Receiver {
    pub fn new(profile: Profile, spec: AudioSpec) -> Self {
        let pulses: SizedPulses = profile.pulses.into_sized(&spec);
        let buffer: NormSamples = NormSamples::new();
        let bits: Vec<u8> = Vec::new();
        let resolver: RxResolver = RxResolver::new();
        let magnitude: FourierMagnitude = FourierMagnitude::new(&pulses, &spec);
        let st_idx: Option<usize> = None;
        Receiver {
            profile,
            pulses,
            spec,
            bits,
            buffer,
            resolver,
            magnitude,
            st_idx,
        }
    }

    pub fn from_file<P>(profile: Profile, filename: P) -> Self
    where
        P: AsRef<Path>,
    {
        let (mut buffer, spec) = read_wav_file(filename);
        buffer.normalize(1.0, 0.1);

        let pulses: SizedPulses = profile.pulses.into_sized(&spec);
        let bits: Vec<u8> = Vec::new();
        let resolver: RxResolver = RxResolver::new();
        let magnitude: FourierMagnitude = FourierMagnitude::new(&pulses, &spec);
        let st_idx: Option<usize> = None;

        Self {
            profile,
            pulses,
            spec,
            bits,
            buffer,
            resolver,
            magnitude,
            st_idx,
        }
    }

    pub fn add_samples(&mut self, samples: &mut NormSamples) {
        samples.normalize(1.0, 0.1);
        self.buffer.0.append(&mut samples.0);
    }

    pub fn analyze_buffer(&mut self) {
        let tone_size: usize = self.pulses.tone_size();

        if let Some(st_idx) = self.st_idx {
            if self.buffer.0.len() > (st_idx + tone_size) {
                self.read_ahead(st_idx);
            }
        } else {
            if self.buffer.0.len() >= (tone_size * 8) {
                if let Some(st_idx) = self.find_start_idx() {
                    self.set_st_idx(st_idx);
                    println!("# Detected Start Signal");
                } else {
                    self.refresh_all_states();
                }
            }
        }
    }

    pub fn save_buffer(&self, filename: &str) {
        self.buffer.save_file(filename, &self.spec);
    }
}

impl Receiver {
    fn set_st_idx(&mut self, idx: usize) {
        self.st_idx = Some(idx);
    }

    fn unset_st_idx(&mut self) {
        self.st_idx = None;
    }

    fn refresh_all_states(&mut self) {
        self.refresh_buffer();
        self.bits.clear();
        self.resolver.reset();
        self.unset_st_idx();
    }

    fn refresh_buffer(&mut self) {
        if let Some(st_idx) = self.st_idx {
            self.drain_buffer_to_start_index(st_idx)
        } else {
            let idx: usize = self.buffer.0.len() - (self.pulses.tone_size() * 8);
            self.drain_buffer_to_start_index(idx);
        }
    }

    fn drain_buffer_to_start_index(&mut self, idx: usize) {
        if idx < self.buffer.0.len() {
            self.buffer.0.drain(..idx);
        } else {
            self.buffer.0.clear();
        }
    }

    fn read_ahead(&mut self, mut st_idx: usize) {
        let tone_size: usize = self.pulses.tone_size();
        let gap_size: usize = self.pulses.gap_size();
        let size_to_next: usize = tone_size + gap_size;

        while (st_idx + tone_size) < self.buffer.0.len() {
            match self.receive_bits(st_idx) {
                RxOutput::Bit(bit) => {
                    self.bits.push(bit);
                    print!("# Bits Received: {}  \r", self.bits.len());
                }
                RxOutput::End => {
                    let string: String = bits_to_string(&self.bits);
                    println!("\n# Decoded Bits: {}\n", string);
                    return self.refresh_all_states();
                }
                RxOutput::Error => {
                    return self.refresh_all_states();
                }
                RxOutput::Undefined => {}
            }

            st_idx += size_to_next;
            self.set_st_idx(st_idx);
        }
    }

    fn find_start_idx(&mut self) -> Option<usize> {
        let mut curr_best_idx: Option<usize> = None;
        let mut curr_best_magnitude: Option<f32> = None;
        let mut consecutive_fails: usize = 0;
        let max_consecutive_fails: usize = 5;

        let mut st_idx: usize = 0;
        let skip_cycles: usize = 8;
        let tone_size: usize = self.pulses.tone_size();

        while st_idx < (self.buffer.0.len() - tone_size) {
            self.re_normalize_pulse_sized_samples(st_idx);
            let samples: &[f32] = self.get_pulse_sized_samples(st_idx);
            let start_magnitude: f32 = self.get_start_magnitude(samples);

            let terminate: bool = self.start_idx_search(
                st_idx,
                start_magnitude,
                &mut curr_best_idx,
                &mut curr_best_magnitude,
                &mut consecutive_fails,
                max_consecutive_fails,
            );

            if terminate {
                break;
            }
            self.update_start_idx(&mut st_idx, skip_cycles, &curr_best_magnitude);
        }
        curr_best_idx
    }

    fn start_idx_search(
        &self,
        idx: usize,
        start_magnitude: f32,
        curr_best_idx: &mut Option<usize>,
        curr_best_magnitude: &mut Option<f32>,
        consecutive_fails: &mut usize,
        max_consecutive_fails: usize,
    ) -> bool {
        match curr_best_magnitude {
            Some(previous_best_magnitude) => {
                if start_magnitude >= *previous_best_magnitude && start_magnitude <= DB_THRESHOLD {
                    *consecutive_fails = 0;
                    *curr_best_idx = Some(idx);
                    *curr_best_magnitude = Some(start_magnitude);
                } else {
                    if *consecutive_fails == max_consecutive_fails {
                        return true;
                    }
                    *consecutive_fails += 1;
                }
            }
            None => {
                if start_magnitude >= -DB_THRESHOLD && start_magnitude <= DB_THRESHOLD {
                    *curr_best_idx = Some(idx);
                    *curr_best_magnitude = Some(start_magnitude);
                }
            }
        }
        false
    }

    fn update_start_idx(&self, idx: &mut usize, cycles: usize, curr_best_magnitude: &Option<f32>) {
        if curr_best_magnitude.is_none() {
            let frequency: f32 = self.profile.markers.start.hz();
            let idx_skip: usize = self.get_minimum_chunk_size(frequency, cycles);
            *idx += idx_skip;
        } else {
            *idx += 1;
        }
    }

    fn receive_bits(&mut self, st_idx: usize) -> RxOutput {
        self.re_normalize_pulse_sized_samples(st_idx);
        let samples: &[f32] = self.get_pulse_sized_samples(st_idx);
        let magnitudes: RxMagnitudes = self.get_magnitudes(samples);
        let output: RxOutput = self.resolver.resolve(&magnitudes);
        output
    }

    fn get_start_magnitude(&self, samples: &[f32]) -> f32 {
        let frequency: f32 = self.profile.markers.start.hz();
        let magnitude: f32 = self.magnitude.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_end_magnitude(&self, samples: &[f32]) -> f32 {
        let frequency: f32 = self.profile.markers.end.hz();
        let magnitude: f32 = self.magnitude.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_next_magnitude(&self, samples: &[f32]) -> f32 {
        let frequency: f32 = self.profile.markers.next.hz();
        let magnitude: f32 = self.magnitude.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_high_magnitude(&self, samples: &[f32]) -> f32 {
        let frequency: f32 = self.profile.bits.high.hz();
        let magnitude: f32 = self.magnitude.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_low_magnitude(&self, samples: &[f32]) -> f32 {
        let frequency: f32 = self.profile.bits.low.hz();
        let magnitude: f32 = self.magnitude.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_magnitudes(&self, samples: &[f32]) -> RxMagnitudes {
        let start_magnitude: f32 = self.get_start_magnitude(samples);
        let end_magnitude: f32 = self.get_end_magnitude(samples);
        let next_magnitude: f32 = self.get_next_magnitude(samples);
        let high_magnitude: f32 = self.get_high_magnitude(samples);
        let low_magnitude: f32 = self.get_low_magnitude(samples);

        let magnitudes: RxMagnitudes = RxMagnitudes::new(
            start_magnitude,
            end_magnitude,
            next_magnitude,
            high_magnitude,
            low_magnitude,
            DB_THRESHOLD,
        );

        // print_detected_magnitudes(&magnitudes);
        magnitudes
    }

    fn get_minimum_chunk_size(&self, frequency: f32, cycles: usize) -> usize {
        let time_for_one_cycle: f32 = 1.0 / frequency;
        let chunk_time: f32 = cycles as f32 * time_for_one_cycle;
        (chunk_time * self.spec.sample_rate() as f32).ceil() as usize
    }

    fn get_pulse_sized_samples<'a>(&'a self, st_idx: usize) -> &'a [f32] {
        let en_idx: usize = self.get_pulse_sized_en_idx(st_idx);
        &self.buffer.0[st_idx..en_idx]
    }

    fn get_mut_pulse_sized_samples<'a>(&'a mut self, st_idx: usize) -> &'a mut [f32] {
        let en_idx: usize = self.get_pulse_sized_en_idx(st_idx);
        &mut self.buffer.0[st_idx..en_idx]
    }

    fn re_normalize_pulse_sized_samples<'a>(&'a mut self, st_idx: usize) {
        let samples: &mut [f32] = self.get_mut_pulse_sized_samples(st_idx);

        let mut normalizer: Normalizer<'_> = Normalizer::new(samples);
        normalizer.normalize_floor(1.0, 0.1);
    }

    fn get_pulse_sized_en_idx(&self, st_idx: usize) -> usize {
        let en_idx: usize = st_idx + self.pulses.tone_size();
        if en_idx > self.buffer.0.len() {
            return self.buffer.0.len();
        }
        en_idx
    }
}

#[allow(dead_code)]
fn print_detected_magnitudes(magnitudes: &RxMagnitudes) {
    let fields: [(&str, f32); 5] = [
        ("Start", magnitudes.start),
        ("End", magnitudes.end),
        ("High", magnitudes.high),
        ("Low", magnitudes.low),
        ("Next", magnitudes.next),
    ];

    let mut printed: bool = false;
    for (label, value) in fields.iter() {
        if magnitudes.within_threshold(*value) {
            if printed {
                print!(" | ");
            }
            print!("{}: {:.2} dB", label, value);
            printed = true;
        }
    }

    if printed {
        println!();
    }
}
