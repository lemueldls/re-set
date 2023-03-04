//! A regex parser and compiler.

pub mod state;

use std::collections::HashMap;

use state::{CasePattern, Program, StateMachine, StepCase};

/// A parsed regex program.
#[derive(Debug)]
pub struct ProgramPatterns {
    /// A list of steps, each containing a list of cases.
    pub steps: Vec<(usize, Vec<StepCase>)>,
    /// A map of steps to the pattern index they should return.
    pub ends: HashMap<usize, usize>,
}

impl ProgramPatterns {
    pub fn new(program: &Program) -> Self {
        let mut state = StateMachine::default();

        let mut insts = state.parse_inst(program.skip(0), program);

        insts.sort_by_key(|(position, _)| *position);
        insts.dedup_by_key(|(position, _)| *position);

        let mut steps = state.extend_steps(insts);

        let ends = state.end_patterns();

        let mut step_map = HashMap::new();

        let step_keys: Vec<_> = steps.iter().map(|(position, _)| *position).collect();

        let mut keys: Vec<_> = step_keys.iter().chain(ends.keys()).collect();
        keys.sort_unstable();

        for (index, position) in keys.iter().enumerate() {
            step_map.insert(**position, index);
        }

        for (position, step_cases) in &mut steps {
            *position = step_map[position];

            for step_case in step_cases {
                if let CasePattern::Step(next_step, condition) = &mut step_case.next_case {
                    *next_step = step_map[next_step];

                    for (position, _) in condition {
                        *position = step_map[position];
                    }
                }
            }
        }

        let ends = ends
            .into_iter()
            .map(|(position, index)| (step_map[&position], index))
            .collect();

        ProgramPatterns { steps, ends }
    }

    pub fn first_step(&self) -> usize {
        let (position, _) = self.steps.first().unwrap();

        *position
    }

    pub fn last_step(&self) -> usize {
        let (position, _) = self.steps.last().unwrap();

        *position
    }

    pub fn step_size(&self) -> u8 {
        let last_step = self.last_step();

        if last_step == 0 {
            8
        } else {
            2_u8.pow((last_step.ilog2() / 8) + 3)
        }
    }
}
