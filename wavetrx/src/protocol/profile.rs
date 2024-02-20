use std::ops::Div;
use std::ops::Mul;
use std::time::Duration;

use crate::audio::types::AudioSpec;

#[derive(Copy, Clone)]
pub struct Frequency(f32);

impl Frequency {
    pub fn hz(&self) -> f32 {
        self.0
    }
}

#[derive(Copy, Clone)]
pub struct PulseDuration(Duration);

impl PulseDuration {
    pub fn from_duration(duration: Duration) -> Self {
        Self(duration)
    }

    pub fn from_nanos<T>(nanos: T) -> Self
    where
        T: Into<u64>,
    {
        Self::from_duration(Duration::from_nanos(T::into(nanos)))
    }

    pub fn from_micros<T>(micros: T) -> Self
    where
        T: Into<u64>,
    {
        Self::from_duration(Duration::from_micros(T::into(micros)))
    }

    pub fn from_millis<T>(millis: T) -> Self
    where
        T: Into<u64>,
    {
        Self::from_duration(Duration::from_millis(T::into(millis)))
    }

    pub fn from_secs<T>(secs: T) -> Self
    where
        T: Into<u64>,
    {
        Self::from_duration(Duration::from_secs(T::into(secs)))
    }

    pub fn as_nanos<T>(&self) -> T
    where
        T: TryFrom<u128>,
    {
        T::try_from(self.0.as_nanos()).ok().unwrap()
    }

    pub fn as_micros<T>(&self) -> T
    where
        T: TryFrom<u128>,
    {
        T::try_from(self.0.as_micros()).ok().unwrap()
    }

    pub fn as_millis<T>(&self) -> T
    where
        T: TryFrom<u128>,
    {
        T::try_from(self.0.as_millis()).ok().unwrap()
    }

    pub fn as_secs<T>(&self) -> T
    where
        T: TryFrom<u64>,
    {
        T::try_from(self.0.as_secs()).ok().unwrap()
    }

    pub fn sample_size<T>(&self, sample_rate: T) -> T
    where
        T: TryFrom<u128> + Mul + Div + From<<T as Mul>::Output> + From<<T as Div>::Output>,
        <T as Mul>::Output: Div<T>,
    {
        let duration: T = self.as_micros::<T>();
        let factor: T = T::try_from(1_000_000).ok().unwrap();

        let sample_size: T = (sample_rate * duration).into();
        let sample_size: T = (sample_size / factor).into();

        sample_size
    }
}

impl Into<PulseDuration> for Duration {
    fn into(self) -> PulseDuration {
        PulseDuration::from_duration(self)
    }
}

#[derive(Copy, Clone)]
pub struct Markers {
    pub start: Frequency,
    pub end: Frequency,
    pub next: Frequency,
}

impl Markers {
    pub fn new(start: f32, end: f32, next: f32) -> Self {
        let start: Frequency = Frequency(start);
        let end: Frequency = Frequency(end);
        let next: Frequency = Frequency(next);
        Self { start, end, next }
    }
}

#[derive(Copy, Clone)]
pub struct Bits {
    pub high: Frequency,
    pub low: Frequency,
}

impl Bits {
    pub fn new(high: f32, low: f32) -> Self {
        let high: Frequency = Frequency(high);
        let low: Frequency = Frequency(low);
        Self { high, low }
    }

    pub fn from_boolean(&self, bit: bool) -> Frequency {
        match bit {
            true => self.high,
            false => self.low,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Pulses {
    pub tone: PulseDuration,
    pub gap: PulseDuration,
}

impl Pulses {
    fn get_tone_sample_size(&self, spec: &AudioSpec) -> usize {
        let sample_rate: usize = spec.sample_rate() as usize;
        let sample_size: usize = self.tone.sample_size::<usize>(sample_rate);
        sample_size
    }

    fn get_gap_sample_size(&self, spec: &AudioSpec) -> usize {
        let sample_rate: usize = spec.sample_rate() as usize;
        let sample_size: usize = self.gap.sample_size::<usize>(sample_rate);
        sample_size
    }
}

impl Pulses {
    pub fn new(tone: Duration, gap: Duration) -> Self {
        let tone: PulseDuration = tone.into();
        let gap: PulseDuration = gap.into();
        Self { tone, gap }
    }

    pub fn into_sized(&self, spec: &AudioSpec) -> SizedPulses {
        let tone_size: usize = self.get_tone_sample_size(spec);
        let gap_size: usize = self.get_gap_sample_size(spec);

        SizedPulses {
            tone_size,
            gap_size,
        }
    }
}

#[derive(Copy, Clone)]
pub struct SizedPulses {
    tone_size: usize,
    gap_size: usize,
}

impl SizedPulses {
    pub fn tone_size(&self) -> usize {
        self.tone_size
    }

    pub fn gap_size(&self) -> usize {
        self.gap_size
    }
}

#[derive(Copy, Clone)]
pub struct Profile {
    pub markers: Markers,
    pub bits: Bits,
    pub pulses: Pulses,
}

impl Profile {
    pub fn new(markers: Markers, bits: Bits, pulses: Pulses) -> Self {
        Profile {
            markers,
            bits,
            pulses,
        }
    }

    pub fn min_frequency_separation(&self, spec: &AudioSpec) -> f32 {
        let sample_rate: f32 = spec.sample_rate() as f32;
        let tone_micros: f32 = self.pulses.tone.as_micros::<u128>() as f32;

        let sample_size: f32 = (sample_rate * tone_micros) / 1e6;
        let min_freq_sep: f32 = sample_rate / sample_size;
        min_freq_sep
    }
}

impl core::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[Profile]\n")?;
        f.write_str("-Markers-\n")?;
        f.write_str(&format!(
            "Start: {:?} Hz\nEnd: {:?} Hz\nNext: {:?} Hz\n",
            self.markers.start.0, self.markers.end.0, self.markers.next.0
        ))?;

        f.write_str("\n-Bits-\n")?;
        f.write_str(&format!(
            "High: {:?} Hz\nLow: {:?} Hz\n",
            self.bits.high.0, self.bits.low.0
        ))?;

        f.write_str("\n-Pulses-\n")?;
        f.write_str(&format!(
            "Tone: {}μs\nGap: {}μs\n",
            self.pulses.tone.0.as_micros(),
            self.pulses.gap.0.as_micros()
        ))?;

        Ok(())
    }
}
