//! A regex parser and compiler.

mod extend;
pub mod parse;

use std::collections::{BTreeMap, HashMap};

use parse::{Program, StepCase};

/// A state machine that can be used to match a regex.
#[derive(Default, Debug)]
pub struct StateMachine {
    /// A list of steps, each containing a list of cases.
    pub steps: BTreeMap<usize, Vec<StepCase>>,
    /// A map of steps to the pattern index they should return.
    pub ends: HashMap<usize, usize>,
}

impl StateMachine {
    #[inline]
    #[must_use]
    pub fn new(program: &Program) -> Self {
        let mut state_machine = Self::default();
        state_machine.parse(program);

        state_machine
    }

    /// # Panics
    ///
    /// Panics if the state machine is empty.
    #[inline]
    #[must_use]
    pub fn first_step(&self) -> usize {
        let (&position, _) = self
            .steps
            .first_key_value()
            .expect("state machine should not be empty");

        position
    }

    /// # Panics
    ///
    /// Panics if the state machine is empty.
    #[inline]
    #[must_use]
    pub fn last_step(&self) -> usize {
        let (&position, _) = self
            .steps
            .last_key_value()
            .expect("state machine should not be empty");

        position
    }

    #[inline]
    #[must_use]
    pub fn step_size(&self) -> u8 {
        let last_step = self.last_step();

        if last_step == 0 {
            8
        } else {
            2_u8.pow((last_step.ilog2() / 8) + 3)
        }
    }
}
