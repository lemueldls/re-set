use std::{collections::HashMap, fmt, ops::RangeInclusive};

use regex::internal::Inst;
pub use regex::internal::{Compiler, Program};

#[derive(Clone)]
pub struct StepCase {
    pub char_range: RangeInclusive<u8>,
    pub next_case: CasePattern,
}

impl fmt::Debug for StepCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} => {:?}", self.char_range, self.next_case)
    }
}

#[derive(Clone)]
pub enum CasePattern {
    Step(usize),
    Match(usize),
}

impl fmt::Debug for CasePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Step(next_step) => write!(f, "step = {next_step}"),
            Self::Match(match_index) => write!(f, "return {match_index}"),
        }
    }
}

/// A state machine that can be used to match a regex.
#[derive(Default)]
pub struct StateMachine {
    /// The next step to be added to the state machine.
    next_step: usize,
    /// A map of split instructions to the step they should skip.
    split_skips: HashMap<usize, usize>,
    /// A map of steps to the pattern index they should return.
    end_patterns: HashMap<usize, usize>,
}

impl StateMachine {
    /// Parse a regex program into a state machine.
    ///
    /// # Panics
    ///
    /// Panics if the program contains a [`Inst::EmptyLook`] instruction.
    pub fn parse_inst(
        &mut self,
        position: usize,
        program: &Program,
    ) -> Vec<(usize, Vec<StepCase>)> {
        match &program[position] {
            Inst::Split(inst_split) => {
                let goto1 = program.skip(inst_split.goto1);
                let goto2 = program.skip(inst_split.goto2);

                let current_step = self.next_step;

                if let Inst::Match(match_index) = program[goto1] {
                    self.end_patterns.insert(current_step, match_index);
                }

                if let Inst::Match(match_index) = program[goto2] {
                    self.end_patterns.insert(current_step, match_index);
                }

                if self.split_skips.contains_key(&position) {
                    return vec![];
                }

                let mut first_steps = Vec::new();
                let mut steps = Vec::new();

                self.split_skips.insert(position, current_step);

                let mut steps1 = self.parse_inst(goto1, program).into_iter();

                self.split_skips.remove(&position);

                if let Some((_, step_cases)) = steps1.next() {
                    first_steps.extend(step_cases);

                    steps.extend(steps1);
                }

                self.split_skips.insert(position, current_step);

                let mut steps2 = self.parse_inst(goto2, program).into_iter();

                self.split_skips.remove(&position);

                if let Some((_, step_cases)) = steps2.next() {
                    first_steps.extend(step_cases);

                    steps.extend(steps2);
                }

                steps.insert(0, (current_step, first_steps.clone()));

                steps
            }
            Inst::Match(..) => vec![],
            Inst::Save(..) => unreachable!(),
            Inst::EmptyLook(..) => todo!(),
            Inst::Char(inst_char) => {
                let mut steps = vec![(
                    self.next_step,
                    vec![StepCase {
                        char_range: (inst_char.c as u8)..=(inst_char.c as u8),
                        next_case: self.next_case(inst_char.goto, program),
                    }],
                )];

                steps.extend(self.parse_inst(program.skip(inst_char.goto), program));

                steps
            }
            Inst::Ranges(inst_ranges) => {
                let mut steps = vec![(self.next_step, Vec::new())];

                for (start, end) in inst_ranges.ranges.iter() {
                    let (_, step_cases) = &mut steps[0];

                    step_cases.push(StepCase {
                        char_range: (*start as u8)..=(*end as u8),
                        next_case: self.next_case(inst_ranges.goto, program),
                    });

                    steps.extend(self.parse_inst(program.skip(inst_ranges.goto), program));
                }

                steps
            }
            Inst::Bytes(inst_bytes) => {
                let mut steps = vec![(
                    self.next_step,
                    vec![StepCase {
                        char_range: inst_bytes.start..=inst_bytes.end,
                        next_case: self.next_case(inst_bytes.goto, program),
                    }],
                )];

                steps.extend(self.parse_inst(program.skip(inst_bytes.goto), program));

                steps
            }
        }
    }

    fn next_case(&mut self, goto: usize, program: &Program) -> CasePattern {
        let goto = program.skip(goto);

        if let Inst::Match(match_index) = program[goto] {
            CasePattern::Match(match_index)
        } else {
            if let Some(next_step) = self.split_skips.get(&goto) {
                return CasePattern::Step(*next_step);
            }

            self.next_step += 1;

            CasePattern::Step(self.next_step)
        }
    }

    #[must_use]
    pub fn end_patterns(self) -> HashMap<usize, usize> {
        self.end_patterns
    }

    pub fn extend_steps(
        &mut self,
        steps: Vec<(usize, Vec<StepCase>)>,
    ) -> Vec<(usize, Vec<StepCase>)> {
        let mut steps1 = steps.clone();
        let steps2 = steps.clone();

        for (step, mut step_cases) in steps {
            extend_split_steps(step, &mut step_cases, &mut steps1, &steps2);
        }

        steps1
    }
}

fn extend_split_steps(
    step: usize,
    first_steps: &mut [StepCase],
    steps1: &mut [(usize, Vec<StepCase>)],
    steps2: &[(usize, Vec<StepCase>)],
) {
    for (i, step_case1) in first_steps.iter().enumerate() {
        for step_case2 in first_steps.iter().skip(i + 1) {
            let overlaps = step_case1
                .char_range
                .clone()
                .any(|c1| step_case2.char_range.clone().any(|c2| c1 == c2));

            if overlaps {
                if let (CasePattern::Step(next_step1), CasePattern::Step(next_step2)) =
                    (&step_case1.next_case, &step_case2.next_case)
                {
                    if next_step1 != next_step2 && *next_step2 != step {
                        if let Some((step1, step_cases1)) =
                            steps1.iter_mut().find(|(step, _)| step == next_step1)
                        {
                            if *next_step1 != step {
                                if let Some((_, step_cases2)) =
                                    steps2.iter().find(|(step, _)| step == next_step2)
                                {
                                    step_cases1.extend(step_cases2.clone());

                                    extend_split_steps(
                                        *step1,
                                        &mut step_cases1.clone(),
                                        steps1,
                                        steps2,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
