#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RxState {
    Start,
    End,
    Next,
    Bit,
    Unset,
}

impl RxState {
    pub fn is_start(&self) -> bool {
        if self == &RxState::Start {
            true
        } else {
            false
        }
    }

    pub fn is_bit(&self) -> bool {
        if self == &RxState::Bit {
            true
        } else {
            false
        }
    }

    pub fn is_start_or_bit(&self) -> bool {
        if self.is_start() || self.is_bit() {
            true
        } else {
            false
        }
    }

    pub fn is_next(&self) -> bool {
        if self == &RxState::Next {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RxOutput {
    Bit(u8),
    End,
    Error,
    Undefined,
}

pub struct RxMagnitudes {
    pub start: f32,
    pub end: f32,
    pub next: f32,
    pub high: f32,
    pub low: f32,
    pub threshold: f32,
}

impl RxMagnitudes {
    pub fn new(start: f32, end: f32, next: f32, high: f32, low: f32, threshold: f32) -> Self {
        RxMagnitudes {
            start,
            end,
            next,
            high,
            low,
            threshold,
        }
    }

    pub fn prominent_bit(&self) -> u8 {
        (self.high > self.low) as u8
    }

    pub fn prominent_bit_magnitude(&self) -> f32 {
        if self.prominent_bit() == 1 {
            self.high
        } else {
            self.low
        }
    }

    pub fn within_threshold(&self, value: f32) -> bool {
        value >= -self.threshold && value <= self.threshold
    }

    pub fn within_threshold_from_state(&self, state: &RxState) -> bool {
        let value: f32 = match state {
            RxState::Start => self.start,
            RxState::End => self.end,
            RxState::Next => self.next,
            RxState::Bit => self.prominent_bit_magnitude(),
            RxState::Unset => return false,
        };
        self.within_threshold(value)
    }
}

#[derive(Debug)]
pub struct RxMarker {
    marker: (RxState, RxState),
}

impl RxMarker {
    pub fn new() -> Self {
        let marker: (RxState, RxState) = (RxState::Unset, RxState::Unset);
        Self { marker }
    }

    pub fn with_expectation(expectation: RxState) -> Self {
        let marker: (RxState, RxState) = (RxState::Unset, expectation);
        Self { marker }
    }

    pub fn selection(&self) -> &RxState {
        &self.marker.0
    }

    pub fn expectation(&self) -> &RxState {
        &self.marker.1
    }

    pub fn set_selection(&mut self, state: RxState) {
        self.marker.0 = state;
    }

    pub fn set_expectation(&mut self, state: RxState) {
        self.marker.1 = state;
    }

    pub fn unset_selection(&mut self) {
        self.marker.0 = RxState::Unset;
    }

    pub fn unset_expectation(&mut self) {
        self.marker.1 = RxState::Unset;
    }
}

#[derive(Debug)]
pub struct RxResolver {
    c_marker: RxMarker,
    e_marker: RxMarker,
}

impl RxResolver {
    pub fn new() -> Self {
        let c_marker: RxMarker = RxMarker::with_expectation(RxState::Start);
        let e_marker: RxMarker = RxMarker::new();

        RxResolver { c_marker, e_marker }
    }

    pub fn resolve(&mut self, magnitudes: &RxMagnitudes) -> RxOutput {
        let has_expectation: bool = self.evaluate_expectation(magnitudes);
        let has_end: bool = self.evaluate_end(magnitudes);

        if let Some(end_resolve) = self.resolve_end(magnitudes, has_expectation, has_end) {
            return end_resolve;
        }

        if has_expectation {
            self.update_expectation();

            let selection: &RxState = self.c_marker.selection();
            let expectation: &RxState = self.c_marker.expectation();

            if selection.is_bit() && expectation.is_next() {
                let bit: u8 = magnitudes.prominent_bit();
                return RxOutput::Bit(bit);
            }
        } else if !has_expectation && !has_end {
            return RxOutput::Error;
        }
        RxOutput::Undefined
    }

    pub fn reset(&mut self) {
        self.c_marker.unset_selection();
        self.c_marker.set_expectation(RxState::Start);
        self.e_marker.unset_selection();
        self.e_marker.unset_expectation();
    }
}

impl RxResolver {
    fn evaluate_expectation(&mut self, magnitudes: &RxMagnitudes) -> bool {
        magnitudes.within_threshold_from_state(&self.c_marker.expectation())
    }

    fn update_expectation(&mut self) {
        let expectation: &RxState = self.c_marker.expectation();

        if expectation.is_start_or_bit() {
            self.c_marker.set_selection(*expectation);
            self.c_marker.set_expectation(RxState::Next);
        } else if expectation.is_next() {
            if self.c_marker.selection().is_start_or_bit() {
                self.c_marker.set_expectation(RxState::Bit);
            }
        }
    }

    fn resolve_end(
        &mut self,
        magnitudes: &RxMagnitudes,
        has_expectation: bool,
        has_end: bool,
    ) -> Option<RxOutput> {
        if !has_end {
            let end_expectation: &RxState = self.e_marker.expectation();
            let has_end_expectation: bool = magnitudes.within_threshold_from_state(end_expectation);
            if has_end_expectation && !has_expectation {
                return Some(RxOutput::End);
            }

            self.e_marker.unset_selection();
            self.e_marker.unset_expectation();
        }
        None
    }

    fn evaluate_end(&mut self, magnitudes: &RxMagnitudes) -> bool {
        let expectation: &RxState = self.c_marker.expectation();
        if expectation.is_bit() {
            if self.c_marker.selection().is_bit() {
                if magnitudes.within_threshold_from_state(&RxState::End) {
                    self.e_marker.set_selection(RxState::End);
                    self.e_marker.set_expectation(RxState::Next);
                    return true;
                }
            }
        }
        false
    }
}
