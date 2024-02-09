use std::time::Duration;

pub const AUDIO_BPS: usize = 32;
pub const AUDIO_SR: usize = 48_000;

pub const SAMPLE_SIZE: f32 = (AUDIO_SR as f32 * PULSE_LENGTH_US.as_micros() as f32) / 1_000_000.0;
pub const MIN_FREQ_SEP: f32 = AUDIO_SR as f32 / SAMPLE_SIZE;

pub const LP_FILTER: f32 = 18_000.0;
pub const HP_FILTER: f32 = 200.0;

// Protocol Profile
pub const MARKER_TONE_START: f32 = 7_000.0;
pub const MARKER_TONE_END: f32 = 9_000.0;
pub const MARKER_TONE_NEXT: f32 = 3_000.0;

pub const BIT_TONE_HIGH: f32 = 5_000.0;
pub const BIT_TONE_LOW: f32 = 1_000.0;

pub const PULSE_LENGTH_US: Duration = Duration::from_micros(1_000);
pub const PULSE_GAP_US: Duration = Duration::from_micros(2_000);

// DETECTION THRESHOLD
pub const DB_THRESHOLD: f32 = 8.0;

// pub const AUDIO_BPS: usize = 32;
// pub const AUDIO_SR: usize = 48_000;

// pub const SAMPLE_SIZE: f32 = (AUDIO_SR as f32 * PULSE_LENGTH_US.as_micros() as f32) / 1_000_000.0;
// pub const MIN_FREQ_SEP: f32 = AUDIO_SR as f32 / SAMPLE_SIZE;

// pub const LP_FILTER: f32 = 18_000.0;
// pub const HP_FILTER: f32 = 1_000.0;

// // Protocol Profile
// pub const MARKER_TONE_START: f32 = 5_500.0;
// pub const MARKER_TONE_END: f32 = 6_000.0;
// pub const MARKER_TONE_NEXT: f32 = 5_000.0;

// pub const BIT_TONE_HIGH: f32 = 4_000.0;
// pub const BIT_TONE_LOW: f32 = 4_500.0;

// pub const PULSE_LENGTH_US: Duration = Duration::from_micros(10_000);
// pub const PULSE_GAP_US: Duration = Duration::from_micros(5_000);

// // DETECTION THRESHOLD
// pub const DB_THRESHOLD: f32 = 8.0;
