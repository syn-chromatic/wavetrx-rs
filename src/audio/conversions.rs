use hound::SampleFormat;
use hound::WavSpec;

use crate::audio::types::SampleEncoding;
use crate::audio::types::AudioSpec;

impl From<WavSpec> for AudioSpec {
    fn from(value: WavSpec) -> Self {
        let sr: u32 = value.sample_rate;
        let bps: u16 = value.bits_per_sample;
        let channels: u16 = value.channels;
        let encoding: SampleEncoding = value.sample_format.into();

        let spec: AudioSpec = AudioSpec::new(sr, bps, channels, encoding);
        spec
    }
}

impl From<AudioSpec> for WavSpec {
    fn from(value: AudioSpec) -> Self {
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
            SampleFormat::Float => SampleEncoding::F32,
            SampleFormat::Int => SampleEncoding::I32,
        }
    }
}

impl From<SampleEncoding> for SampleFormat {
    fn from(value: SampleEncoding) -> Self {
        match value {
            SampleEncoding::F32 => SampleFormat::Float,
            SampleEncoding::I32 => SampleFormat::Int,
        }
    }
}
