//! A regex parser and compiler.

pub mod state;

use std::collections::HashMap;

use state::{Program, StateMachine, StepCase};

/// A parsed regex program.
#[derive(Debug)]
pub struct ParsedProgram {
    /// A list of steps, each containing a list of cases.
    pub steps: Vec<(usize, Vec<StepCase>)>,
    /// A map of steps to the pattern index they should return.
    pub ends: HashMap<usize, usize>,
}

/// Parse a regex program into a state machine.
#[inline]
#[must_use]
pub fn parse_program(program: &Program) -> ParsedProgram {
    let mut state = StateMachine::default();

    let insts = state.parse_inst(program.skip(0), program);
    let steps = state.extend_steps(insts);

    let ends = state.end_patterns();

    ParsedProgram { steps, ends }
}
