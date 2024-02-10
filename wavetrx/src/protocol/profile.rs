use std::ops::Div;
use std::ops::Mul;
use std::time::Duration;

#[derive(Copy, Clone)]
pub struct Frequency(f32);

impl Frequency {
    pub fn hz(&self) -> f32 {
        self.0
    }
}

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

pub struct Pulses {
    pub tone: PulseDuration,
    pub gap: PulseDuration,
}

impl Pulses {
    pub fn new(tone: Duration, gap: Duration) -> Self {
        let tone: PulseDuration = tone.into();
        let gap: PulseDuration = gap.into();
        Self { tone, gap }
    }
}

pub struct ProtocolProfile {
    pub markers: Markers,
    pub bits: Bits,
    pub pulses: Pulses,
}

impl ProtocolProfile {
    pub fn new(markers: Markers, bits: Bits, pulses: Pulses) -> Self {
        ProtocolProfile {
            markers,
            bits,
            pulses,
        }
    }
}
