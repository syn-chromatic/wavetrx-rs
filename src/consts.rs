pub const AUDIO_BPS: usize = 32;
pub const AUDIO_SR: usize = 48_000;
pub const TONE_LENGTH_US: usize = 1000;
pub const TONE_GAP_US: usize = 500;

pub const SAMPLE_SIZE: f32 = (AUDIO_SR as f32 * TONE_LENGTH_US as f32) / 1_000_000.0;
pub const MIN_FREQ_SEP: f32 = AUDIO_SR as f32 / SAMPLE_SIZE;

pub const LP_FILTER: f32 = 20_000.0;
pub const HP_FILTER: f32 = 12_000.0;

pub const BIT_FREQUENCY_ON: f32 = 14_000.0;
pub const BIT_FREQUENCY_OFF: f32 = 15_000.0;
pub const BIT_FREQUENCY_NEXT: f32 = 16_000.0;

pub const TRANSMIT_START_FREQUENCY: f32 = 17_000.0;
pub const TRANSMIT_END_FREQUENCY: f32 = 18_000.0;

pub const DB_THRESHOLD: f32 = 8.0;
