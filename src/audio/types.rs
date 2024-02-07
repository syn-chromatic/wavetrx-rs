#[derive(Clone, Copy)]
pub enum SampleEncoding {
    Float,
    Int,
}

#[derive(Clone, Copy)]
pub struct SampleSpec {
    sr: u32,
    bps: u16,
    channels: u16,
    encoding: SampleEncoding,
}

impl SampleSpec {
    pub fn new(sr: u32, bps: u16, channels: u16, encoding: SampleEncoding) -> Self {
        Self {
            channels,
            sr,
            bps,
            encoding,
        }
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.sr
    }

    pub fn bits_per_sample(&self) -> u16 {
        self.bps
    }

    pub fn encoding(&self) -> SampleEncoding {
        self.encoding
    }
}
