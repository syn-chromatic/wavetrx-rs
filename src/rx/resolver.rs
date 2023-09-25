use crate::rx::states::{RxMagnitudes, RxOutput, RxStates};

#[derive(Debug)]
pub struct RxResolver {
    selection: Option<RxStates>,
    expectation: RxStates,
    end_selection: Option<RxStates>,
    end_expectation: Option<RxStates>,
}

impl RxResolver {
    pub fn new() -> Self {
        let selection: Option<RxStates> = None;
        let expectation: RxStates = RxStates::Start;
        let end_selection: Option<RxStates> = None;
        let end_expectation: Option<RxStates> = None;
        RxResolver {
            selection,
            expectation,
            end_selection,
            end_expectation,
        }
    }

    pub fn resolve(&mut self, magnitudes: &RxMagnitudes) -> Option<RxOutput> {
        let end_evaluation: bool = self.evaluate_end(magnitudes);
        let evaluation: bool = magnitudes.evaluate(&self.expectation);

        let end_resolve: Option<RxOutput> =
            self.resolve_end(magnitudes, end_evaluation, evaluation);
        if end_resolve.is_some() {
            return end_resolve;
        }

        if evaluation {
            self.set_expectation();

            if self.expectation == RxStates::Next {
                if let Some(selection) = &self.selection {
                    if *selection == RxStates::Bit {
                        let bit: u8 = magnitudes.get_bit();
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
        self.selection = None;
        self.expectation = RxStates::Start;
        self.end_selection = None;
        self.end_expectation = None;
    }
}

impl RxResolver {
    fn set_expectation(&mut self) {
        if self.expectation == RxStates::Start || self.expectation == RxStates::Bit {
            self.selection = Some(self.expectation.clone());
            self.expectation = RxStates::Next;
        } else if self.expectation == RxStates::Next {
            if let Some(selection) = &self.selection {
                if *selection == RxStates::Start || *selection == RxStates::Bit {
                    self.expectation = RxStates::Bit;
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
            if let Some(end_expectation) = &self.end_expectation {
                let end_evaluation = magnitudes.evaluate(end_expectation);
                if end_evaluation && !evaluation {
                    return Some(RxOutput::End);
                } else {
                    self.end_selection = None;
                    self.end_expectation = None;
                }
            }
        }
        None
    }

    fn evaluate_end(&mut self, magnitudes: &RxMagnitudes) -> bool {
        if self.expectation == RxStates::Bit {
            if let Some(selection) = &self.selection {
                if *selection == RxStates::Bit {
                    if magnitudes.evaluate(&RxStates::End) {
                        self.end_selection = Some(RxStates::End);
                        self.end_expectation = Some(RxStates::Next);
                        return true;
                    }
                }
            }
        }
        false
    }
}
