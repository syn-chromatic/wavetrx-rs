use std::time::Duration;

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

// // CONFIG 1
// Protocol Profile
// pub const MARKER_TONE_START: f32 = 5_500.0;
// pub const MARKER_TONE_END: f32 = 6_000.0;
// pub const MARKER_TONE_NEXT: f32 = 5_000.0;

// pub const BIT_TONE_HIGH: f32 = 4_000.0;
// pub const BIT_TONE_LOW: f32 = 4_500.0;

// pub const PULSE_LENGTH_US: Duration = Duration::from_micros(10_000);
// pub const PULSE_GAP_US: Duration = Duration::from_micros(5_000);

// // DETECTION THRESHOLD
// pub const DB_THRESHOLD: f32 = 8.0;

// #

// // CONFIG 2
// Protocol Profile
// pub const MARKER_TONE_START: f32 = 7_000.0;
// pub const MARKER_TONE_END: f32 = 9_000.0;
// pub const MARKER_TONE_NEXT: f32 = 3_000.0;

// pub const BIT_TONE_HIGH: f32 = 5_000.0;
// pub const BIT_TONE_LOW: f32 = 1_000.0;

// pub const PULSE_LENGTH_US: Duration = Duration::from_micros(1_000);
// pub const PULSE_GAP_US: Duration = Duration::from_micros(2_000);

// // DETECTION THRESHOLD
// pub const DB_THRESHOLD: f32 = 8.0;
