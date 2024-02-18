#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RxState {
    Start,
    End,
    Next,
    Bit,
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
        };
        self.within_threshold(value)
    }
}

#[derive(Debug)]
pub struct RxCurrentMarker {
    marker: (Option<RxState>, RxState),
}

impl RxCurrentMarker {
    pub fn new() -> Self {
        let marker: (Option<RxState>, RxState) = (None, RxState::Start);
        Self { marker }
    }

    pub fn selection(&self) -> &Option<RxState> {
        &self.marker.0
    }

    pub fn expectation(&self) -> &RxState {
        &self.marker.1
    }

    pub fn set_selection(&mut self, state: RxState) {
        self.marker.0 = Some(state);
    }

    pub fn set_expectation(&mut self, state: RxState) {
        self.marker.1 = state;
    }

    pub fn unset_selection(&mut self) {
        self.marker.0 = None;
    }

    pub fn reset(&mut self) {
        self.marker.0 = None;
        self.marker.1 = RxState::Start;
    }
}

#[derive(Debug)]
pub struct RxEndMarker {
    marker: (Option<RxState>, Option<RxState>),
}

impl RxEndMarker {
    pub fn new() -> Self {
        let marker: (Option<RxState>, Option<RxState>) = (None, None);
        Self { marker }
    }

    pub fn selection(&self) -> &Option<RxState> {
        &self.marker.0
    }

    pub fn expectation(&self) -> &Option<RxState> {
        &self.marker.1
    }

    pub fn set_selection(&mut self, state: RxState) {
        self.marker.0 = Some(state);
    }

    pub fn set_expectation(&mut self, state: RxState) {
        self.marker.1 = Some(state);
    }

    pub fn unset_selection(&mut self) {
        self.marker.0 = None;
    }

    pub fn unset_expectation(&mut self) {
        self.marker.1 = None;
    }

    pub fn reset(&mut self) {
        self.marker.0 = None;
        self.marker.1 = None;
    }
}

#[derive(Debug)]
pub struct RxResolver {
    c_marker: RxCurrentMarker,
    e_marker: RxEndMarker,
}

impl RxResolver {
    pub fn new() -> Self {
        let current_marker: RxCurrentMarker = RxCurrentMarker::new();
        let end_marker: RxEndMarker = RxEndMarker::new();

        RxResolver {
            c_marker: current_marker,
            e_marker: end_marker,
        }
    }

    pub fn resolve(&mut self, magnitudes: &RxMagnitudes) -> Option<RxOutput> {
        let evaluation: bool =
            magnitudes.within_threshold_from_state(&self.c_marker.expectation());
        let end_evaluation: bool = self.evaluate_end(magnitudes);

        if let Some(end_resolve) = self.resolve_end(magnitudes, end_evaluation, evaluation) {
            return Some(end_resolve);
        }

        if evaluation {
            self.set_expectation();

            if self.c_marker.expectation().is_next() {
                if let Some(selection) = self.c_marker.selection() {
                    if selection.is_bit() {
                        let bit: u8 = magnitudes.prominent_bit();
                        return Some(RxOutput::Bit(bit));
                    }
                }
            }
        } else if !evaluation && !end_evaluation {
            return Some(RxOutput::Error);
        }
        None
    }

    pub fn reset(&mut self) {
        self.c_marker.reset();
        self.e_marker.reset();
    }
}

impl RxResolver {
    fn set_expectation(&mut self) {
        let expectation: &RxState = self.c_marker.expectation();

        if expectation.is_start_or_bit() {
            self.c_marker.set_selection(*expectation);
            self.c_marker.set_expectation(RxState::Next);
        } else if expectation.is_next() {
            if let Some(selection) = self.c_marker.selection() {
                if selection.is_start_or_bit() {
                    self.c_marker.set_expectation(RxState::Bit);
                }
            }
        }
    }

    fn resolve_end(
        &mut self,
        magnitudes: &RxMagnitudes,
        end_evaluation: bool,
        evaluation: bool,
    ) -> Option<RxOutput> {
        if !end_evaluation {
            if let Some(end_expectation) = &self.e_marker.expectation() {
                let end_evaluation = magnitudes.within_threshold_from_state(end_expectation);
                if end_evaluation && !evaluation {
                    return Some(RxOutput::End);
                } else {
                    self.e_marker.unset_selection();
                    self.e_marker.unset_expectation();
                }
            }
        }
        None
    }

    fn evaluate_end(&mut self, magnitudes: &RxMagnitudes) -> bool {
        let expectation: &RxState = self.c_marker.expectation();
        if expectation.is_bit() {
            if let Some(selection) = self.c_marker.selection() {
                if *selection == RxState::Bit {
                    if magnitudes.within_threshold_from_state(&RxState::End) {
                        self.e_marker.set_selection(RxState::End);
                        self.e_marker.set_expectation(RxState::Next);
                        return true;
                    }
                }
            }
        }
        false
    }
}
