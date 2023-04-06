use std::{
    collections::{HashMap, HashSet},
    fmt,
    ops::RangeInclusive,
};

use regex::internal::Inst;
pub use regex::internal::{Compiler, Program};

#[derive(Clone)]
pub struct StepCase {
    pub byte_range: RangeInclusive<u8>,
    pub next_case: CasePattern,
}

impl fmt::Debug for StepCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}..={} => {:?}",
            self.byte_range.start(),
            self.byte_range.end(),
            self.next_case
        )
    }
}
#[derive(Clone)]
pub enum CasePattern {
    Step(usize, Vec<(usize, RangeInclusive<u8>)>),
    Match(usize),
}

impl fmt::Debug for CasePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Step(next_step, _) => write!(f, "step = {next_step}"),
            Self::Match(match_index) => write!(f, "return {match_index}"),
        }
    }
}

/// A state machine that can be used to match a regex.
#[derive(Default)]
pub struct StateMachine {
    split_skips: HashSet<usize>,
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

                self.split_match(position, program);

                let mut first_steps = Vec::new();
                let mut steps = Vec::new();

                if self.split_skips.contains(&position) {
                    return vec![];
                }

                if position != goto1 {
                    self.split_skips.insert(position);
                }

                let steps1 = self.parse_inst(goto1, program);

                if let Some((_, step_cases)) = steps1.first() {
                    first_steps.extend(step_cases.clone());

                    steps.extend(steps1);
                }

                self.split_skips.remove(&goto2);

                let steps2 = self.parse_inst(goto2, program);

                if let Some((_, step_cases)) = steps2.first() {
                    first_steps.extend(step_cases.clone());

                    steps.extend(steps2);
                }

                steps.insert(0, (position, first_steps.clone()));

                steps
            }
            Inst::Match(match_index) => {
                self.end_patterns.insert(position, *match_index);

                vec![]
            }
            Inst::Save(inst_save) => unreachable!("{inst_save:?}"),
            Inst::EmptyLook(..) => todo!(),
            Inst::Char(inst_char) => {
                let mut steps = vec![(
                    position,
                    vec![StepCase {
                        byte_range: (inst_char.c as u8)..=(inst_char.c as u8),
                        next_case: self.next_case(inst_char.goto, program),
                    }],
                )];

                steps.extend(self.parse_inst(program.skip(inst_char.goto), program));

                steps
            }
            Inst::Ranges(inst_ranges) => {
                let mut steps = vec![(position, Vec::new())];

                for (start, end) in inst_ranges.ranges.iter() {
                    let (_, step_cases) = &mut steps[0];

                    step_cases.push(StepCase {
                        byte_range: (*start as u8)..=(*end as u8),
                        next_case: self.next_case(inst_ranges.goto, program),
                    });

                    steps.extend(self.parse_inst(program.skip(inst_ranges.goto), program));
                }

                steps
            }
            Inst::Bytes(inst_bytes) => {
                let mut steps = vec![(
                    position,
                    vec![StepCase {
                        byte_range: inst_bytes.start..=inst_bytes.end,
                        next_case: self.next_case(inst_bytes.goto, program),
                    }],
                )];

                steps.extend(self.parse_inst(program.skip(inst_bytes.goto), program));

                steps
            }
        }
    }

    fn split_match(&mut self, position: usize, program: &Program) {
        let mut next_splits = vec![position];

        while let Some(next_split) = next_splits.pop() {
            if let Inst::Split(inst_split) = &program[next_split] {
                let goto1 = program.skip(inst_split.goto1);
                let goto2 = program.skip(inst_split.goto2);

                if let Inst::Match(match_index) = program[goto1] {
                    self.end_patterns.insert(position, match_index);
                }
                if let Inst::Match(match_index) = program[goto2] {
                    self.end_patterns.insert(position, match_index);
                }

                next_splits.push(goto1);
                next_splits.push(goto2);
            }
        }
    }

    fn next_case(&mut self, goto: usize, program: &Program) -> CasePattern {
        let goto = program.skip(goto);

        if let Inst::Match(match_index) = program[goto] {
            CasePattern::Match(match_index)
        } else {
            CasePattern::Step(goto, Vec::new())
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

        let (step, step_cases) = steps.into_iter().next().unwrap();

        self.extend_split_steps(step, step_cases, &mut steps1, &steps2, &mut HashSet::new());

        steps1
    }

    fn extend_split_steps(
        &mut self,
        position: usize,
        first_steps: Vec<StepCase>,
        steps1: &mut [(usize, Vec<StepCase>)],
        steps2: &[(usize, Vec<StepCase>)],
        exclude: &mut HashSet<(usize, usize)>,
    ) {
        for (i, step_case1) in first_steps.iter().enumerate() {
            for step_case2 in first_steps.iter().skip(i + 1) {
                let overlaps = step_case1
                    .byte_range
                    .clone()
                    .any(|byte| step_case2.byte_range.contains(&byte));

                if overlaps {
                    match (&step_case1.next_case, &step_case2.next_case) {
                        (CasePattern::Step(next_step1, _), CasePattern::Step(next_step2, _)) => {
                            if next_step1 != next_step2
                                && *next_step1 != position
                                && !exclude.contains(&(*next_step1, *next_step2))
                            {
                                if let Some(match_index) = self.end_patterns.get(next_step2) {
                                    self.add_condition(
                                        steps1,
                                        position,
                                        i,
                                        match_index,
                                        step_case2,
                                    );
                                }

                                if let Some((step1, step_cases1)) =
                                    steps1.iter_mut().find(|(step, _)| step == next_step1)
                                {
                                    if let Some((_, step_cases2)) =
                                        steps2.iter().find(|(step, _)| step == next_step2)
                                    {
                                        step_cases1.extend(step_cases2.clone());

                                        exclude.insert((*next_step1, *next_step2));

                                        self.extend_split_steps(
                                            *step1,
                                            step_cases1.clone(),
                                            steps1,
                                            steps2,
                                            exclude,
                                        );
                                    }
                                }
                            }
                        }
                        (_, CasePattern::Match(match_index)) => {
                            self.add_condition(steps1, position, i, match_index, step_case2);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn add_condition(
        &self,
        steps1: &mut [(usize, Vec<StepCase>)],
        position: usize,
        i: usize,
        match_index: &usize,
        step_case2: &StepCase,
    ) {
        let (_, step_cases) = steps1
            .iter_mut()
            .find(|(step, _)| step == &position)
            .unwrap();

        let step_case1 = &mut step_cases[i];

        if let CasePattern::Step(_, condition) = &mut step_case1.next_case {
            let (step, _) = self
                .end_patterns
                .iter()
                .find(|(_, index)| index == &match_index)
                .unwrap();

            condition.push((*step, step_case2.byte_range.clone()));
        }
    }
}
