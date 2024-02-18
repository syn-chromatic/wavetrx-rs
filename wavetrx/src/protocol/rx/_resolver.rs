#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RxState {
    Start,
    End,
    Next,
    Bit,
    Unset,
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
            RxState::Unset => return false,
        };
        self.within_threshold(value)
    }
}

#[derive(Debug)]
pub struct RxMarker {
    selection: RxState,
    expectation: RxState,
}

impl RxMarker {
    pub fn new() -> Self {
        let selection: RxState = RxState::Unset;
        let expectation: RxState = RxState::Unset;

        Self {
            selection,
            expectation,
        }
    }

    pub fn with_expectation(expectation: RxState) -> Self {
        let selection: RxState = RxState::Unset;

        Self {
            selection,
            expectation,
        }
    }

    pub fn selection_match(&self, state: &RxState) -> bool {
        &self.selection == state
    }

    pub fn expectation_match(&self, state: &RxState) -> bool {
        &self.expectation == state
    }

    pub fn set_selection(&mut self, selection: RxState) {
        self.selection = selection;
    }

    pub fn set_expectation(&mut self, expectation: RxState) {
        self.expectation = expectation;
    }

    pub fn unset_selection(&mut self) {
        self.selection = RxState::Unset;
    }

    pub fn unset_expectation(&mut self) {
        self.expectation = RxState::Unset;
    }
}

#[derive(Debug)]
pub struct RxResolver {
    current_marker: RxMarker,
    end_marker: RxMarker,
}

impl RxResolver {
    pub fn new() -> Self {
        let current_marker: RxMarker = RxMarker::with_expectation(RxState::Start);
        let end_marker: RxMarker = RxMarker::new();

        Self {
            current_marker,
            end_marker,
        }
    }

    pub fn resolve(&mut self, magnitudes: &RxMagnitudes) -> RxOutput {
        let evaluation: bool =
            magnitudes.within_threshold_from_state(&self.current_marker.expectation);
        let end_evaluation: bool = self.evaluate_end(magnitudes);

        if let Some(end_resolve) = self.resolve_end(magnitudes, end_evaluation, evaluation) {
            return end_resolve;
        }

        if evaluation {
            self.set_expectation();

            if self.current_marker.expectation_match(&RxState::Next) {
                if self.current_marker.selection_match(&RxState::Bit) {
                    let bit: u8 = magnitudes.prominent_bit();
                    return RxOutput::Bit(bit);
                }
            }
        }

        RxOutput::Error
    }

    pub fn reset(&mut self) {
        self.current_marker.unset_selection();
        self.current_marker.set_expectation(RxState::Start);

        self.end_marker.unset_selection();
        self.end_marker.unset_expectation();
    }
}

impl RxResolver {
    fn set_expectation(&mut self) {
        if self.current_marker.expectation_match(&RxState::Start)
            || self.current_marker.expectation_match(&RxState::Bit)
        {
            self.current_marker
                .set_selection(self.current_marker.expectation.clone());
            self.current_marker.set_expectation(RxState::Next);
        } else if self.current_marker.expectation_match(&RxState::Next) {
            if self.current_marker.selection_match(&RxState::Start)
                || self.current_marker.selection_match(&RxState::Bit)
            {
                self.current_marker.set_expectation(RxState::Bit);
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
            let end_evaluation =
                magnitudes.within_threshold_from_state(&self.end_marker.expectation);

            if end_evaluation && !evaluation {
                return Some(RxOutput::End);
            } else {
                self.end_marker.unset_selection();
                self.end_marker.unset_expectation();
            }
        }
        None
    }

    fn evaluate_end(&mut self, magnitudes: &RxMagnitudes) -> bool {
        if self.current_marker.expectation_match(&RxState::Bit) {
            if self.current_marker.selection_match(&RxState::Bit) {
                let evaluation: bool = magnitudes.within_threshold_from_state(&RxState::End);
                if evaluation {
                    self.end_marker.set_selection(RxState::End);
                    self.end_marker.set_expectation(RxState::Next);
                    return true;
                }
            }
        }
        false
    }
}
