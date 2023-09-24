pub const AUDIO_BPS: usize = 16;
pub const AUDIO_SR: usize = 48_000;
pub const TONE_LENGTH_US: usize = 10_000;
pub const TONE_GAP_US: usize = 10_000;

pub const SAMPLE_SIZE: f32 = (AUDIO_SR as f32 * TONE_LENGTH_US as f32) / 1_000_000.0;
pub const MIN_FREQ_SEP: f32 = AUDIO_SR as f32 / SAMPLE_SIZE;

pub const LP_FILTER: f32 = 20_000.0;
pub const HP_FILTER: f32 = 18_800.0;

pub const BIT_FREQUENCY_ON: f32 = 19_000.0;
pub const BIT_FREQUENCY_OFF: f32 = 19_200.0;
pub const BIT_FREQUENCY_NEXT: f32 = 19_400.0;

pub const TRANSMIT_START_FREQUENCY: f32 = 19_600.0;
pub const TRANSMIT_END_FREQUENCY: f32 = 19_800.0;

pub const SAMPLING_MAGNITUDE: f32 = ((2usize.pow(AUDIO_BPS as u32 - 1)) - 1) as f32;
pub const DB_THRESHOLD: f32 = 8.0;
