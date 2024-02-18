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

    pub fn within_threshold(&self, magnitudes: &RxMagnitudes) -> bool {
        let value: f32 = match self {
            RxState::Start => magnitudes.start,
            RxState::End => magnitudes.end,
            RxState::Next => magnitudes.next,
            RxState::Bit => magnitudes.prominent_bit_magnitude(),
            RxState::Unset => return false,
        };
        magnitudes.within_threshold(value)
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
        let initial_expectation: bool = self.evaluate_expectation(magnitudes);
        let has_end: bool = self.evaluate_end(magnitudes);

        if let Some(resolve) = self.resolve_end(magnitudes, initial_expectation, has_end) {
            return resolve;
        }

        if let Some(resolve) = self.resolve_expectation(magnitudes, initial_expectation) {
            return resolve;
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
    fn resolve_expectation(
        &mut self,
        magnitudes: &RxMagnitudes,
        initial_expectation: bool,
    ) -> Option<RxOutput> {
        if initial_expectation {
            let selection: &RxState = self.c_marker.selection();
            let expectation: &RxState = self.c_marker.expectation();

            if selection.is_bit() && expectation.is_next() {
                let bit: u8 = magnitudes.prominent_bit();
                return Some(RxOutput::Bit(bit));
            }
        }
        None
    }

    fn evaluate_expectation(&mut self, magnitudes: &RxMagnitudes) -> bool {
        let expectation: &RxState = self.c_marker.expectation();
        if expectation.within_threshold(magnitudes) {
            if expectation.is_start_or_bit() {
                self.c_marker.set_selection(*expectation);
                self.c_marker.set_expectation(RxState::Next);
            } else if expectation.is_next() {
                if self.c_marker.selection().is_start_or_bit() {
                    self.c_marker.set_expectation(RxState::Bit);
                }
            }
            return true;
        }
        false
    }

    fn resolve_end(
        &mut self,
        magnitudes: &RxMagnitudes,
        initial_expectation: bool,
        has_end: bool,
    ) -> Option<RxOutput> {
        if !has_end {
            let expectation: &RxState = self.e_marker.expectation();
            if !initial_expectation && expectation.within_threshold(magnitudes) {
                return Some(RxOutput::End);
            }

            self.e_marker.unset_selection();
            self.e_marker.unset_expectation();
        }
        if !initial_expectation && !has_end {
            return Some(RxOutput::Error);
        }
        None
    }

    fn evaluate_end(&mut self, magnitudes: &RxMagnitudes) -> bool {
        let expectation: &RxState = self.c_marker.expectation();
        if expectation.is_bit() {
            if self.c_marker.selection().is_bit() {
                if RxState::End.within_threshold(magnitudes) {
                    self.e_marker.set_selection(RxState::End);
                    self.e_marker.set_expectation(RxState::Next);
                    return true;
                }
            }
        }
        false
    }
}
