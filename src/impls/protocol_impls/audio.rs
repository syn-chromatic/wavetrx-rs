use hound::SampleFormat;
use hound::WavSpec;

use crate::protocol::SampleEncoding;
use crate::protocol::SampleSpec;

impl From<WavSpec> for SampleSpec {
    fn from(value: WavSpec) -> Self {
        let sr: u32 = value.sample_rate;
        let bps: u16 = value.bits_per_sample;
        let channels: u16 = value.channels;
        let encoding: SampleEncoding = value.sample_format.into();

        let spec: SampleSpec = SampleSpec::new(sr, bps, channels, encoding);
        spec
    }
}

impl From<SampleSpec> for WavSpec {
    fn from(value: SampleSpec) -> Self {
        let channels: u16 = value.channels();
        let sample_rate: u32 = value.sample_rate();
        let bits_per_sample: u16 = value.bits_per_sample();
        let sample_format: SampleFormat = value.encoding().into();

        let spec: WavSpec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample,
            sample_format,
        };
        spec
    }
}

impl From<SampleFormat> for SampleEncoding {
    fn from(value: SampleFormat) -> Self {
        match value {
            SampleFormat::Float => SampleEncoding::Float,
            SampleFormat::Int => SampleEncoding::Int,
        }
    }
}

impl From<SampleEncoding> for SampleFormat {
    fn from(value: SampleEncoding) -> Self {
        match value {
            SampleEncoding::Float => SampleFormat::Float,
            SampleEncoding::Int => SampleFormat::Int,
        }
    }
}
