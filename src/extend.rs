use std::collections::HashSet;

use crate::{parse::CasePattern, StateMachine};

impl StateMachine {
    /// # Panics
    ///
    /// Panics if the state machine is empty.
    #[inline]
    pub fn extend_steps(&mut self) {
        let mut mut_steps = self.steps.clone();

        let start = *mut_steps
            .first_entry()
            .expect("state machine should not be empty")
            .key();

        self.extend_split_steps(start, &mut HashSet::new());
    }

    fn extend_split_steps(&mut self, current_step: usize, exclude: &mut HashSet<(usize, usize)>) {
        let first_steps = self.steps[&current_step].clone();

        for (step_index, step_case) in first_steps.iter().enumerate() {
            for other_step_case in first_steps.iter().skip(step_index + 1) {
                let other_byte_range = other_step_case.byte_range;

                let overlaps = step_case.byte_range.0 <= other_byte_range.1
                    && step_case.byte_range.1 >= other_byte_range.0;

                if overlaps {
                    match (&step_case.next_case, &other_step_case.next_case) {
                        (
                            &CasePattern::Step(next_step, _),
                            &CasePattern::Step(other_next_step, _),
                        ) => {
                            if next_step != other_next_step
                                && next_step != current_step
                                && !exclude.contains(&(next_step, other_next_step))
                            {
                                if let Some(&match_index) = self.ends.get(&other_next_step) {
                                    self.add_condition(
                                        current_step,
                                        step_index,
                                        match_index,
                                        other_byte_range,
                                    );
                                };

                                let ref_steps = self.steps[&other_next_step].clone();
                                self.steps
                                    .get_mut(&next_step)
                                    .expect("missing step")
                                    .extend(ref_steps);

                                exclude.insert((next_step, other_next_step));

                                self.extend_split_steps(next_step, exclude);
                            }
                        }
                        (_, &CasePattern::Match(match_index)) => {
                            self.add_condition(
                                current_step,
                                step_index,
                                match_index,
                                other_byte_range,
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn add_condition(
        &mut self,
        current_step: usize,
        step_index: usize,
        match_index: usize,
        byte_range: (u8, u8),
    ) {
        let (_, step_cases) = self
            .steps
            .iter_mut()
            .find(|(&step, _)| step == current_step)
            .expect("`current_step` should exist in `steps`");

        let step_case = &mut step_cases[step_index];

        if let CasePattern::Step(_, condition) = &mut step_case.next_case {
            let (&step, _) = self
                .ends
                .iter()
                .find(|(_, &index)| index == match_index)
                .expect("`match_index` should exist in `ends`");

            condition.push((step, byte_range));
        }
    }
}
