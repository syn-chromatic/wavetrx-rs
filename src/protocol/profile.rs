pub struct ProtocolProfile {
    pub start: f32,
    pub end: f32,
    pub next: f32,
    pub high: f32,
    pub low: f32,
    pub tone_length: usize,
    pub gap_length: usize,
}

impl ProtocolProfile {
    pub fn new(
        start: f32,
        end: f32,
        next: f32,
        high: f32,
        low: f32,
        tone_length: usize,
        gap_length: usize,
    ) -> Self {
        ProtocolProfile {
            start,
            end,
            next,
            high,
            low,
            tone_length,
            gap_length,
        }
    }
}
