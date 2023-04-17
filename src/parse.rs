use std::{borrow::Cow, fmt};

pub use regex::internal::{Compiler, Inst, Program};

use crate::StateMachine;

#[derive(Clone)]
pub struct StepCase {
    pub byte_range: (u8, u8),
    pub next_case: CasePattern,
}

impl fmt::Debug for StepCase {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (start, end) = self.byte_range;

        write!(f, "{start}..={end} => {:?}", self.next_case)
    }
}
#[derive(Clone)]
pub enum CasePattern {
    Step(usize, Vec<(usize, (u8, u8))>),
    Match(usize),
}

impl fmt::Debug for CasePattern {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Step(next_step, _) => write!(f, "step = {next_step}"),
            Self::Match(match_index) => write!(f, "return {match_index}"),
        }
    }
}
impl StateMachine {
    #[inline]
    pub fn parse(&mut self, program: &Program) {
        for position in 0..program.len() {
            self.parse_inst(program.skip(position), program);
        }

        self.extend_steps();
    }

    /// Parse a regex program into a state machine.
    ///
    /// # Panics
    ///
    /// Panics if the program contains an unsupported instruction.
    #[inline]
    pub fn parse_inst(&mut self, position: usize, program: &Program) -> Cow<[StepCase]> {
        if self.steps.contains_key(&position) {
            return Cow::Borrowed(&self.steps[&position]);
        }

        let step_cases = match &program[position] {
            Inst::Split(inst_split) => {
                let goto1 = program.skip(inst_split.goto1);
                let goto2 = program.skip(inst_split.goto2);

                self.split_match(position, program);

                let mut step_cases = self.parse_inst(goto1, program).into_owned();
                step_cases.extend(self.parse_inst(goto2, program).into_owned());

                Cow::from_iter(step_cases)
            }
            Inst::Match(match_index) => {
                self.ends.insert(position, *match_index);

                Cow::default()
            }
            Inst::Save(inst_save) => unimplemented!("{inst_save:?}"),
            Inst::EmptyLook(inst_empty_look) => unimplemented!("{inst_empty_look:?}"),
            Inst::Char(inst_char) => unimplemented!("{inst_char:?}"),
            Inst::Ranges(inst_ranges) => unimplemented!("{inst_ranges:?}"),
            Inst::Bytes(inst_bytes) => Cow::from_iter([StepCase {
                byte_range: (inst_bytes.start, inst_bytes.end),
                next_case: Self::next_case(inst_bytes.goto, program),
            }]),
        };

        self.steps.insert(position, step_cases.to_vec());

        step_cases
    }

    fn split_match(&mut self, position: usize, program: &Program) {
        let mut next_splits = vec![position];

        while let Some(next_split) = next_splits.pop() {
            if let Inst::Split(inst_split) = &program[next_split] {
                let goto1 = program.skip(inst_split.goto1);
                let goto2 = program.skip(inst_split.goto2);

                if let Inst::Match(match_index) = program[goto1] {
                    self.ends.insert(position, match_index);
                }
                if let Inst::Match(match_index) = program[goto2] {
                    self.ends.insert(position, match_index);
                }

                next_splits.push(goto1);
                next_splits.push(goto2);
            }
        }
    }

    fn next_case(goto: usize, program: &Program) -> CasePattern {
        let goto = program.skip(goto);

        if let Inst::Match(match_index) = program[goto] {
            CasePattern::Match(match_index)
        } else {
            CasePattern::Step(goto, Vec::new())
        }
    }
}
