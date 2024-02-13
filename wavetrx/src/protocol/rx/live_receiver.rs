use super::resolver::RxMagnitudes;
use super::resolver::RxOutput;
use super::resolver::RxResolver;

use crate::audio::filters::FrequencyPass;
use crate::audio::spectrum::FourierMagnitude;
use crate::audio::spectrum::Normalizer;
use crate::audio::types::AudioSpec;
use crate::audio::types::NormSamples;

use crate::profile::ProtocolProfile;
use crate::profile::PulseDuration;
use crate::utils::bits_to_string;
use crate::utils::save_normalized_name;

use crate::consts::DB_THRESHOLD;
use crate::consts::HP_FILTER;
use crate::consts::LP_FILTER;

pub struct LiveReceiver {
    profile: ProtocolProfile,
    buffer: NormSamples,
    bits: Vec<u8>,
    resolver: RxResolver,
    spec: AudioSpec,
    frequency_magnitude: FourierMagnitude,
    sample_size: usize,
    start_idx: Option<usize>,
    start_signal: bool,
}

impl LiveReceiver {
    pub fn new(profile: ProtocolProfile, spec: AudioSpec) -> Self {
        let buffer: NormSamples = NormSamples::new();
        let bits: Vec<u8> = Vec::new();
        let resolver: RxResolver = RxResolver::new();
        let sample_rate: usize = spec.sample_rate() as usize;
        let sample_size: usize = profile.pulses.tone.sample_size::<usize>(sample_rate);
        let frequency_magnitude: FourierMagnitude = FourierMagnitude::new(sample_size, &spec);
        let start_idx: Option<usize> = None;
        let start_signal: bool = false;
        LiveReceiver {
            profile,
            buffer,
            bits,
            resolver,
            spec,
            frequency_magnitude,
            sample_size,
            start_idx,
            start_signal,
        }
    }

    pub fn add_samples(&mut self, samples: &mut NormSamples) {
        let tsz: usize = self.get_tone_sample_size();
        let gsz: usize = self.get_gap_sample_size();

        // self.apply_frequency_filters(samples);
        // self.normalize_samples(samples);

        self.re_normalize_samples_chunk(&mut samples.0);
        self.buffer.0.append(&mut samples.0);

        if self.start_idx.is_some() && self.start_signal {
            let mut start_idx: usize = self.start_idx.unwrap();
            if self.buffer.0.len() > start_idx + self.sample_size {
                while start_idx + self.sample_size < self.buffer.0.len() {
                    let output: Option<RxOutput> = self.receive_bits(start_idx);
                    if let Some(output) = output {
                        self.handle_rx_output(output);
                    }
                    start_idx += tsz + gsz;
                    self.start_idx = Some(start_idx);
                }
            }
        } else {
            if self.buffer.0.len() >= self.sample_size * 8 {
                let start_idx: Option<usize> = self.find_starting_index();
                if start_idx.is_some() {
                    self.start_idx = start_idx;
                    self.start_signal = true;
                    println!("# Detected Start Signal");
                } else {
                    self.refresh_all_states();
                }
            }
        }
    }

    pub fn get_sample_size(&self) -> usize {
        self.sample_size
    }

    pub fn save(&self, filename: &str) {
        save_normalized_name(filename, &self.buffer.0, &self.spec);
    }
}

impl LiveReceiver {
    fn refresh_all_states(&mut self) {
        self.refresh_buffer();
        self.bits.clear();
        self.resolver.reset();
        self.start_idx = None;
        self.start_signal = false;
    }

    fn refresh_buffer(&mut self) {
        if let Some(idx) = self.start_idx {
            self.drain_buffer_to_starting_index(idx)
        } else {
            let idx: usize = self.buffer.0.len() - (self.sample_size * 8);
            self.drain_buffer_to_starting_index(idx);
        }
    }

    fn drain_buffer_to_starting_index(&mut self, idx: usize) {
        if idx < self.buffer.0.len() {
            self.buffer.0.drain(..idx);
        } else {
            self.buffer.0.clear();
        }
    }

    fn get_timestamp(&self, idx: usize, sample_rate: usize) -> f32 {
        let timestamp = idx as f32 / sample_rate as f32;
        timestamp
    }

    fn apply_frequency_filters(&self, samples: &mut [f32]) {
        let highpass_frequency: f32 = HP_FILTER;
        let lowpass_frequency: f32 = LP_FILTER;

        let mut filters: FrequencyPass<'_> = FrequencyPass::new(samples, &self.spec);
        filters.apply_highpass(highpass_frequency, 0.707);
        filters.apply_lowpass(lowpass_frequency, 0.707);
    }

    fn handle_rx_output(&mut self, output: RxOutput) {
        match output {
            RxOutput::Bit(bit) => {
                self.bits.push(bit);
                print!("# Bits Received: {}  \r", self.bits.len());
            }
            RxOutput::End => {
                let string: String = bits_to_string(&self.bits);
                println!("\n# Decoded Bits: {}\n", string);
                self.refresh_all_states();
            }
            RxOutput::Error => {
                self.refresh_all_states();
            }
        }
    }

    fn find_starting_index(&self) -> Option<usize> {
        let mut current_best_idx: Option<usize> = None;
        let mut current_best_magnitude: Option<f32> = None;
        let mut consecutive_fails: usize = 0;
        let max_consecutive_fails: usize = 5;

        let freq_mag: &FourierMagnitude = &self.frequency_magnitude;

        let mut idx: usize = 0;
        let skip_cycles: usize = 8;
        let sample_rate: usize = freq_mag.get_sample_rate();
        let sample_size: usize = self.sample_size;
        let samples: &Vec<f32> = &self.buffer.0;

        while idx < (samples.len() - sample_size) {
            let samples_chunk: Vec<f32> = self.get_owned_samples_chunk(samples, idx, sample_size);
            let start_magnitude: f32 = self.get_start_magnitude(&samples_chunk, freq_mag);

            let terminate: bool = self.update_starting_index_search(
                idx,
                start_magnitude,
                &mut current_best_idx,
                &mut current_best_magnitude,
                &mut consecutive_fails,
                max_consecutive_fails,
            );
            if terminate {
                break;
            }
            self.update_starting_index(&mut idx, skip_cycles, sample_rate, &current_best_magnitude);
        }
        current_best_idx
    }

    fn update_starting_index_search(
        &self,
        idx: usize,
        start_magnitude: f32,
        current_best_idx: &mut Option<usize>,
        current_best_magnitude: &mut Option<f32>,
        consecutive_fails: &mut usize,
        max_consecutive_fails: usize,
    ) -> bool {
        match current_best_magnitude {
            Some(previous_best_magnitude) => {
                if start_magnitude >= *previous_best_magnitude && start_magnitude <= DB_THRESHOLD {
                    *consecutive_fails = 0;
                    *current_best_idx = Some(idx);
                    *current_best_magnitude = Some(start_magnitude);
                } else {
                    if *consecutive_fails == max_consecutive_fails {
                        return true;
                    }
                    *consecutive_fails += 1;
                }
            }
            None => {
                if start_magnitude >= -DB_THRESHOLD && start_magnitude <= DB_THRESHOLD {
                    *current_best_idx = Some(idx);
                    *current_best_magnitude = Some(start_magnitude);
                }
            }
        }
        false
    }

    fn update_starting_index(
        &self,
        idx: &mut usize,
        cycles: usize,
        sample_rate: usize,
        current_best_magnitude: &Option<f32>,
    ) {
        if current_best_magnitude.is_none() {
            let frequency: f32 = self.profile.markers.start.hz();
            let idx_skip: usize = self.get_minimum_chunk_size(frequency, cycles, sample_rate);
            *idx += idx_skip;
        } else {
            *idx += 1;
        }
    }

    fn receive_bits(&mut self, idx: usize) -> Option<RxOutput> {
        let samples_chunk: Vec<f32> =
            self.get_owned_samples_chunk(&self.buffer.0, idx, self.sample_size);
        let magnitudes: RxMagnitudes = self.get_magnitudes(&samples_chunk);
        let output: Option<RxOutput> = self.resolver.resolve(&magnitudes);
        output
    }

    fn get_tone_sample_size(&self) -> usize {
        let tone_pulse: &PulseDuration = &self.profile.pulses.tone;

        let sample_rate: usize = self.spec.sample_rate() as usize;
        let sample_size: usize = tone_pulse.sample_size::<usize>(sample_rate);
        sample_size
    }

    fn get_gap_sample_size(&self) -> usize {
        let gap_pulse: &PulseDuration = &self.profile.pulses.gap;

        let sample_rate: usize = self.spec.sample_rate() as usize;
        let sample_size: usize = gap_pulse.sample_size::<usize>(sample_rate);
        sample_size
    }

    fn get_start_magnitude(&self, samples: &[f32], freq_mag: &FourierMagnitude) -> f32 {
        let frequency: f32 = self.profile.markers.start.hz();
        let magnitude: f32 = freq_mag.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_end_magnitude(&self, samples: &[f32], freq_mag: &FourierMagnitude) -> f32 {
        let frequency: f32 = self.profile.markers.end.hz();
        let magnitude: f32 = freq_mag.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_next_magnitude(&self, samples: &[f32], freq_mag: &FourierMagnitude) -> f32 {
        let frequency: f32 = self.profile.markers.next.hz();
        let magnitude: f32 = freq_mag.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_high_magnitude(&self, samples: &[f32], freq_mag: &FourierMagnitude) -> f32 {
        let frequency: f32 = self.profile.bits.high.hz();
        let magnitude: f32 = freq_mag.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_low_magnitude(&self, samples: &[f32], freq_mag: &FourierMagnitude) -> f32 {
        let frequency: f32 = self.profile.bits.low.hz();
        let magnitude: f32 = freq_mag.get_magnitude(samples, frequency);
        magnitude
    }

    fn get_magnitudes(&self, samples: &[f32]) -> RxMagnitudes {
        let start_magnitude: f32 = self.get_start_magnitude(samples, &self.frequency_magnitude);
        let end_magnitude: f32 = self.get_end_magnitude(samples, &self.frequency_magnitude);
        let next_magnitude: f32 = self.get_next_magnitude(samples, &self.frequency_magnitude);
        let high_magnitude: f32 = self.get_high_magnitude(samples, &self.frequency_magnitude);
        let low_magnitude: f32 = self.get_low_magnitude(samples, &self.frequency_magnitude);

        let magnitudes: RxMagnitudes = RxMagnitudes::new(
            start_magnitude,
            end_magnitude,
            next_magnitude,
            high_magnitude,
            low_magnitude,
            DB_THRESHOLD,
        );

        // print_magnitude(&magnitudes);
        magnitudes
    }

    fn get_minimum_chunk_size(&self, frequency: f32, cycles: usize, sample_rate: usize) -> usize {
        let time_for_one_cycle: f32 = 1.0 / frequency;
        let chunk_time: f32 = cycles as f32 * time_for_one_cycle;
        (chunk_time * sample_rate as f32).ceil() as usize
    }

    fn get_owned_samples_chunk<'a>(
        &self,
        samples: &'a [f32],
        idx: usize,
        sample_size: usize,
    ) -> Vec<f32> {
        let en_idx: usize = idx + sample_size;
        let en_idx: usize = self.clamp_ending_index(en_idx);
        let mut samples_chunk: Vec<f32> = samples[idx..en_idx].to_vec();
        self.re_normalize_samples_chunk(&mut samples_chunk);
        samples_chunk
    }

    fn clamp_ending_index(&self, idx: usize) -> usize {
        if idx > self.buffer.0.len() {
            return self.buffer.0.len();
        }
        idx
    }

    fn normalize_samples(&self, samples: &mut [f32]) {
        let bit_depth: usize = self.spec.bits_per_sample() as usize;
        let mut normalizer: Normalizer<'_> = Normalizer::new(samples);
        normalizer.normalize(bit_depth, 0.1);
    }

    fn re_normalize_samples_chunk(&self, chunk: &mut [f32]) {
        let mut normalizer: Normalizer<'_> = Normalizer::new(chunk);
        normalizer.re_normalize(0.1);
    }
}

fn print_magnitude(magnitudes: &RxMagnitudes) {
    let mut boolean: bool = false;

    if magnitudes.within_threshold(magnitudes.start) {
        print!("Start: {:.2} dB", magnitudes.start);
        boolean = true;
    }
    if magnitudes.within_threshold(magnitudes.end) {
        if boolean {
            print!(" | ");
        }
        print!("End: {:.2} dB", magnitudes.end);
        boolean = true;
    }
    if magnitudes.within_threshold(magnitudes.high) {
        if boolean {
            print!(" | ");
        }
        print!("High: {:.2} dB", magnitudes.high);
        boolean = true;
    }
    if magnitudes.within_threshold(magnitudes.low) {
        if boolean {
            print!(" | ");
        }
        print!("Low: {:.2} dB", magnitudes.low);
        boolean = true;
    }
    if magnitudes.within_threshold(magnitudes.next) {
        if boolean {
            print!(" | ");
        }
        print!("Next: {:.2} dB", magnitudes.next);
        boolean = true;
    }

    if boolean {
        println!();
    }
}
