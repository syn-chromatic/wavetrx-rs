use biquad::Biquad;
use biquad::Coefficients;
use biquad::DirectForm1;
use biquad::Hertz;
use biquad::ToHertz;
use biquad::Type;

use super::types::AudioSpec;

pub struct FrequencyPass<'a> {
    samples: &'a mut [f32],
    spec: &'a AudioSpec,
}

impl<'a> FrequencyPass<'a> {
    pub fn new(samples: &'a mut [f32], spec: &'a AudioSpec) -> Self {
        FrequencyPass { samples, spec }
    }

    pub fn apply_highpass(&mut self, frequency: f32, q_value: f32) {
        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            self.get_coefficients(Type::HighPass, frequency, q_value);

        if let Ok(coefficients) = coefficients {
            self.apply_coefficients(coefficients);
        }
    }

    pub fn apply_lowpass(&mut self, frequency: f32, q_value: f32) {
        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            self.get_coefficients(Type::LowPass, frequency, q_value);

        if let Ok(coefficients) = coefficients {
            self.apply_coefficients(coefficients);
        }
    }

    pub fn apply_bandpass(&mut self, lower_frequency: f32, upper_frequency: f32, sharpness: f32) {
        let center_frequency: f32 = (lower_frequency * upper_frequency).sqrt();
        let mut q_value: f32 = center_frequency / (upper_frequency - lower_frequency);
        q_value *= sharpness;

        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            self.get_coefficients(Type::BandPass, center_frequency, q_value);

        if let Ok(coefficients) = coefficients {
            self.apply_coefficients(coefficients);
        }
    }
}

impl<'a> FrequencyPass<'a> {
    fn get_coefficients(
        &self,
        filter: Type,
        frequency: f32,
        q_value: f32,
    ) -> Result<Coefficients<f32>, biquad::Errors> {
        let fs: Hertz<f32> = self.spec.sample_rate().hz();
        let f0: Hertz<f32> = frequency.hz();

        let coefficients: Result<Coefficients<f32>, biquad::Errors> =
            Coefficients::<f32>::from_params(filter, fs, f0, q_value);
        coefficients
    }

    fn apply_coefficients(&mut self, coefficients: Coefficients<f32>) {
        let mut filter: DirectForm1<f32> = DirectForm1::<f32>::new(coefficients);

        for sample in self.samples.iter_mut() {
            *sample = filter.run(*sample);
        }
    }
}

#[test]
fn test_filter() {
    use super::types::NormSamples;
    use super::types::SampleEncoding;
    use hound::WavReader;
    use std::fs::File;
    use std::io::BufReader;

    let filename: &str = "sweep_h.wav";
    let mut reader: WavReader<BufReader<File>> = WavReader::open(filename).unwrap();
    let spec: AudioSpec = reader.spec().into();

    println!("{:?}", spec);

    let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
    let mut samples: NormSamples = NormSamples::from_i32(&samples, &spec);

    let highpass_frequency: f32 = 1000.0;
    let lowpass_frequency: f32 = 5000.0;

    let mut filters: FrequencyPass<'_> = FrequencyPass::new(&mut samples.0, &spec);
    let spec: AudioSpec =
        AudioSpec::new(spec.sample_rate(), 32, spec.channels(), SampleEncoding::F32);

    filters.apply_highpass(highpass_frequency, 1.0);
    filters.apply_lowpass(lowpass_frequency, 0.707);
    // filters.apply_bandpass(5000.0, 10_000.0, 2.0);

    samples.save_file("test_filters2.wav", &spec);
}
