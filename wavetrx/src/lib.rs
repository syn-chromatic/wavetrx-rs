pub mod audio;
pub mod consts;
pub mod profile;
pub mod protocol;
pub mod tests;
pub mod utils;

use crate::profile::Bits;
use crate::profile::Markers;
use crate::profile::ProtocolProfile;
use crate::profile::Pulses;

use crate::consts::{
    BIT_TONE_HIGH, BIT_TONE_LOW, MARKER_TONE_END, MARKER_TONE_NEXT, MARKER_TONE_START,
    PULSE_GAP_US, PULSE_LENGTH_US,
};

pub fn get_profile() -> ProtocolProfile {
    let markers: Markers = Markers::new(MARKER_TONE_START, MARKER_TONE_END, MARKER_TONE_NEXT);
    let bits: Bits = Bits::new(BIT_TONE_HIGH, BIT_TONE_LOW);
    let pulses: Pulses = Pulses::new(PULSE_LENGTH_US, PULSE_GAP_US);

    let profile: ProtocolProfile = ProtocolProfile::new(markers, bits, pulses);
    profile
}
