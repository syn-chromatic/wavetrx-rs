use crate::consts::DB_THRESHOLD;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RxStates {
    Start,
    End,
    Next,
    Bit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RxOutput {
    Bit(u8),
    End,
    Error,
}

pub struct RxMagnitudes {
    pub start: f32,
    pub end: f32,
    pub next: f32,
    pub high: f32,
    pub low: f32,
}

impl RxMagnitudes {
    pub fn new(start: f32, end: f32, next: f32, high: f32, low: f32) -> Self {
        RxMagnitudes {
            start,
            end,
            next,
            high,
            low,
        }
    }

    pub fn evaluate(&self, state: &RxStates) -> bool {
        match state {
            RxStates::Start => self.within_range(self.start),
            RxStates::End => self.within_range(self.end),
            RxStates::Next => self.within_range(self.next),
            RxStates::Bit => self.within_range(self.high) || self.within_range(self.low),
        }
    }

    pub fn within_range(&self, value: f32) -> bool {
        value >= -DB_THRESHOLD && value <= DB_THRESHOLD
    }

    pub fn get_bit(&self) -> u8 {
        if self.high > self.low {
            return 1;
        }
        0
    }
}
