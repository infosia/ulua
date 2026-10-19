//! Count generated statements on each forward path of one Luau function.
use crate::{INTERRUPT_STATEMENTS, STORE_WEIGHT};

#[derive(Clone, Copy, Default)]
struct Flow {
    count: usize,
}
impl Flow {
    fn join(self, other: Self) -> Self {
        Self {
            count: self.count.max(other.count),
        }
    }
}
#[derive(Default)]
pub(super) struct Interrupts {
    flow: Flow,
    frames: Vec<Frame>,
    #[cfg(test)]
    hoisted: Option<usize>,
}
/// A counting decision is consumed by the line writer that can insert its point.
#[must_use]
#[derive(Clone, Copy)]
pub(super) enum Point {
    Here,
    Before(usize),
}
struct Frame {
    entry: Flow,
    largest: Flow,
    kind: Kind,
    has_else: bool,
    head: Option<(usize, usize)>,
}
enum Kind {
    Branch,
    Loop,
    Block,
    Function,
}
impl Interrupts {
    // This classifies fixed emitter delimiters, never user source or expression contents.
    // Execution lines count as statements. Closure arguments belong to their enclosing call.
    #[cfg(test)]
    pub(super) fn before(&mut self, text: &str, minimum_cost: usize) -> bool {
        self.before_at(text, minimum_cost, 0)
    }
    #[cfg(test)]
    pub(super) fn take_hoisted(&mut self) -> Option<usize> {
        self.hoisted.take()
    }
    pub(super) fn checkpoint(&mut self) {
        self.flow = Flow::default();
        self.clear_heads();
    }
    pub(super) fn shift_heads(&mut self, position: usize, bytes: usize) {
        for frame in &mut self.frames {
            if let Some((head, _)) = &mut frame.head
                && *head >= position
            {
                *head += bytes;
            }
        }
    }
    fn clear_heads(&mut self) {
        for frame in self.frames.iter_mut().rev() {
            if matches!(frame.kind, Kind::Function) {
                break;
            }
            frame.head = None;
        }
    }
    // Heads retain the pre-header forward count across ordinary statements.
    // A constant-true header reserves a boundary but executes no forward work;
    // its removable count includes that reservation, as in the first-statement rule.
    fn hoist_head(&mut self, cost: usize) -> Option<(usize, usize)> {
        let function = self
            .frames
            .iter()
            .rposition(|f| matches!(f.kind, Kind::Function))
            .map_or(0, |i| i + 1);
        let index = (function..self.frames.len()).rfind(|i| self.frames[*i].head.is_some())?;
        let (position, before) = self.frames[index].head?;
        let reduced = self.flow.count.checked_sub(before)?;
        if reduced + cost > INTERRUPT_STATEMENTS {
            return None;
        }
        for frame in &mut self.frames[index..] {
            frame.entry.count = frame.entry.count.saturating_sub(before);
            frame.largest.count = frame.largest.count.saturating_sub(before);
        }
        Some((reduced, position))
    }
    // Test adapters preserve the compact flow-state fixtures. Production has one
    // decision consumer, line_counted; no pending move survives a call to count.
    #[cfg(test)]
    pub(super) fn before_at(&mut self, text: &str, cost: usize, position: usize) -> bool {
        self.before_store_at(text, cost, position, None)
    }
    #[cfg(test)]
    pub(super) fn before_store_at(
        &mut self,
        text: &str,
        cost: usize,
        position: usize,
        store: Option<usize>,
    ) -> bool {
        let point = self.count(text, cost, position, store);
        self.hoisted = match point {
            Some(Point::Before(position)) => Some(position),
            _ => None,
        };
        point.is_some()
    }
    pub(super) fn count(
        &mut self,
        text: &str,
        minimum_cost: usize,
        position: usize,
        store_cost: Option<usize>,
    ) -> Option<Point> {
        let text = text.trim();
        if text == "else" {
            if let Some(frame) = self.frames.last_mut() {
                frame.has_else = true;
                frame.largest = frame.largest.join(self.flow);
                self.flow = frame.entry;
            }
            return None;
        }
        if text == "end" || text == "until true" {
            if let Some(frame) = self.frames.pop() {
                self.flow = match frame.kind {
                    Kind::Function => frame.entry,
                    Kind::Block => self.flow,
                    Kind::Branch if frame.has_else => self.flow.join(frame.largest),
                    Kind::Branch | Kind::Loop => self.flow.join(frame.entry).join(frame.largest),
                };
            }
            return None;
        }
        if text == "," || text == "nil" {
            return None;
        }
        if text == ")" || text == ")), 0)" {
            // Protected closure arguments are complete. The enclosing call always executes.
            self.checkpoint();
            return None;
        }
        let argument_function = text.starts_with("function(");
        let function = argument_function || text.starts_with("local function ");
        let block = text == "do";
        let repeat = text == "repeat";
        let branch = text.starts_with("if ") && text.ends_with(" then");
        let loop_head =
            (text.starts_with("while ") || text.starts_with("for ")) && text.ends_with(" do");
        let mut point = None;
        // Reserve work for inline branches and pure groups. Storage paths also need VM field reads.
        let value = text.split_once(" = (").map_or("", |(_, value)| value);
        let base = if value.contains(" and ") || value.contains(" or ") {
            8
        } else if value.starts_with("if ") {
            3
        } else {
            1
        };
        let weight = if table_store(text) {
            STORE_WEIGHT
        } else if value.starts_with("__ulua_tostring(") || text.starts_with("__ulua_tostring(") {
            // FASTCALL removes the conversion's CALL interrupt but retains its string lookup work.
            // Weight one gave a 6.308 step-time ratio for boolean templates against the bound of six.
            2
        } else {
            1
        };
        let cost = store_cost.unwrap_or_else(|| {
            (base + value.matches(".__ulua_").count())
                .max(weight)
                .max(minimum_cost)
                .min(INTERRUPT_STATEMENTS)
        });
        // Reserve loop boundaries before split conditions. Constant true headers add no forward work.
        let executable = !block && !repeat && !argument_function;
        let mut before_head = self.flow.count;
        if executable {
            if self.flow.count + cost > INTERRUPT_STATEMENTS {
                if let Some((count, position)) = self.hoist_head(cost) {
                    self.flow.count = count;
                    point = Some(Point::Before(position));
                } else {
                    self.flow.count = 0;
                    point = Some(Point::Here);
                }
                self.clear_heads();
                before_head = self.flow.count;
            }
            self.flow.count += cost;
            if text == "while true do" {
                before_head = self.flow.count;
            }
        }
        if function || block || repeat || branch || loop_head {
            self.frames.push(Frame {
                entry: self.flow,
                largest: Flow::default(),
                has_else: false,
                head: (loop_head || repeat).then_some((position, before_head)),
                kind: if function {
                    Kind::Function
                } else if block {
                    Kind::Block
                } else if branch {
                    Kind::Branch
                } else {
                    Kind::Loop
                },
            });
            if function {
                self.flow = Flow::default();
            }
        } else if always_calls(text) || text.starts_with("return ") || text.starts_with("do return")
        {
            self.checkpoint();
        }
        if text == "do break end"
            || text == "do continue end"
            || (text.starts_with("if ") && text.ends_with(" then break end"))
        {
            // Save each exit before a later checkpoint can reset the body's count.
            for frame in self.frames.iter_mut().rev() {
                match frame.kind {
                    Kind::Loop => {
                        frame.largest = frame.largest.join(self.flow);
                        break;
                    }
                    Kind::Function => break,
                    Kind::Branch | Kind::Block => {}
                }
            }
        }
        point
    }
}

fn always_calls(text: &str) -> bool {
    // A complete statement can call directly, or assign the result of a direct call.
    // Parenthesized compound expressions and conditional calls retain their count.
    let value = text.split_once(" = (").map_or(text, |(_, value)| value);
    let Some((callee, arguments)) = value.split_once('(') else {
        return false;
    };
    // Native aliases can use FASTCALL, which skips the CALL interrupt.
    // Count all library aliases conservatively, including print and gsub without FASTCALL at this pin.
    (callee.starts_with("__ulua_") || callee == "error")
        && !matches!(
            callee,
            "__ulua_fmod"
                | "__ulua_math_abs"
                | "__ulua_math_sqrt"
                | "__ulua_floor"
                | "__ulua_ceil"
                | "__ulua_tostring"
                | "__ulua_gsub"
                | "__ulua_print"
        )
        && callee
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !arguments.is_empty()
        && !value.ends_with('(')
}

fn table_store(text: &str) -> bool {
    let Some((target, _)) = text.split_once(" = ") else {
        return false;
    };
    target.starts_with("__ulua_") && target.contains(['.', '['])
}

#[cfg(test)]
mod tests {
    use super::Interrupts;
    use crate::INTERRUPT_STATEMENTS;

    // Cost: 154 fixed statement classifications; no native compile or execution.
    #[test]
    fn library_alias_calls_retain_the_forward_count() {
        for (call, count) in [
            ("__ulua_fmod(x, 3)", INTERRUPT_STATEMENTS),
            ("__ulua_floor(x)", INTERRUPT_STATEMENTS),
            ("__ulua_math_abs(x)", INTERRUPT_STATEMENTS),
            ("__ulua_math_sqrt(x)", INTERRUPT_STATEMENTS),
            ("__ulua_ceil(x)", INTERRUPT_STATEMENTS),
            ("__ulua_tostring(x)", INTERRUPT_STATEMENTS / 2),
            ("__ulua_gsub(s, 'x', 'y')", INTERRUPT_STATEMENTS),
            ("__ulua_print(s)", INTERRUPT_STATEMENTS),
        ] {
            for statement in [call.to_owned(), format!("__ulua_t0 = ({call}) :: number")] {
                let mut counter = Interrupts::default();
                for _ in 0..count {
                    assert!(!counter.before(&statement, 1), "{statement}");
                }
                assert!(
                    counter.before("local next = 1", 1),
                    "{statement} erased the count"
                );
            }
        }
    }

    // Cost: 17 fixed classifications. The text carries no operator or closure clues.
    #[test]
    fn explicit_operation_cost_limits_plain_statement_runs() {
        for cost in [2, 3, 11] {
            let mut counter = Interrupts::default();
            for _ in 0..INTERRUPT_STATEMENTS / cost {
                assert!(!counter.before("local value = 0", cost));
            }
            assert!(counter.before("local value = 0", cost));
        }
    }

    // Cost: 17 classifications per form. Conditional traps and native equality have no guaranteed call.
    #[test]
    fn inline_checks_and_equality_retain_the_forward_count() {
        for (line, cost) in [
            (
                "if not (i >= 0 and i < #a and __ulua_floor(i) == i) then __ulua_trap(1) end",
                4,
            ),
            (
                "if not (i >= 0 and i <= #a and __ulua_floor(i) == i) then __ulua_trap(1) end",
                4,
            ),
            ("__ulua_t0 = ((a == b)) :: boolean", 2),
            (
                "local __ulua_g0_1: boolean = not (i >= 0 and i <= #a and __ulua_floor(i) == i)",
                4,
            ),
            ("if __ulua_g0_1 then __ulua_trap(1) end", 1),
        ] {
            let mut counter = Interrupts::default();
            for _ in 0..INTERRUPT_STATEMENTS - cost {
                assert!(!counter.before("local value = 0", 1));
            }
            assert!(!counter.before(line, cost));
            assert!(
                counter.before("local next = 1", 1),
                "{line} erased the count"
            );
        }
    }

    // Cost: one fixed prelude scan; the count comes from declarations, independently of the reservation.
    #[test]
    fn library_local_reservation_matches_declarations() {
        assert_eq!(
            super::super::prelude::LIBRARIES
                .lines()
                .filter(|line| line.starts_with("local "))
                .count(),
            super::super::LIBRARY_LOCALS,
        );
    }
}

#[cfg(test)]
mod loop_head_tests {
    use super::Interrupts;

    // Cost: 23 classifications per exit. Numeric zero-trip and early exits retain their forward work.
    #[test]
    fn numeric_loop_exits_join_the_header_and_early_body_paths() {
        for exit in ["do break end", "if i == bound then break end"] {
            let mut count = Interrupts::default();
            for _ in 0..10 {
                assert!(!count.before("local x = 0", 1));
            }
            assert!(!count.before_at("for i = 0, limit, 1 do", 4, 100));
            assert!(!count.before(exit, 1));
            assert!(!count.before("local first = 1", 1));
            assert!(count.before("local later = 2", 1));
            assert!(!count.before("end", 1));
            assert!(!count.before("local after = 3", 2));
            assert_eq!(count.flow.count, 9);
        }
    }

    // Cost: 19 fixed classifications. Reserve the boundary before a split short-circuit condition.
    #[test]
    fn loop_boundary_keeps_the_first_condition_point_before_the_loop() {
        let mut count = Interrupts::default();
        for _ in 0..15 {
            assert!(!count.before("local x = 0", 1));
        }
        assert!(!count.before_at("while true do", 1, 100));
        assert!(count.before("condition = ((n < 255)) :: boolean", 1));
        assert_eq!(count.take_hoisted(), Some(100));
        assert!(!count.before("do", 1));
        assert!(!count.before("local fresh: boolean = condition", 1));
    }

    // Cost: three classifications. A reserved boundary adds no forward work to a hoisted point.
    #[test]
    fn a_full_weight_first_statement_still_hoists() {
        let mut count = Interrupts::default();
        assert!(!count.before("local x = 0", 1));
        assert!(!count.before_at("while true do", 1, 100));
        assert!(count.before("local closure = function() end", 16));
        assert_eq!(count.take_hoisted(), Some(100));
    }

    // Cost: 38 fixed classifications. No compiler, VM, or clock assertion.
    #[test]
    fn first_body_point_precedes_pending_loop_wrappers() {
        let mut count = Interrupts::default();
        for _ in 0..15 {
            assert!(!count.before("local x = 0", 1));
        }
        assert!(!count.before_at("while true do", 1, 100));
        assert!(!count.before_at("repeat", 1, 120));
        assert!(!count.before("do", 1));
        assert!(count.before("local first = 1", 1));
        assert_eq!(count.take_hoisted(), Some(120));
        for line in ["end", "until true", "end"] {
            assert!(!count.before(line, 1));
        }
        assert!(count.before("local after = 0", 1));
        for _ in 0..15 {
            assert!(!count.before("local after = 0", 1));
        }
        assert!(count.before("local last = 0", 1));
    }

    // Cost: 19 fixed classifications per exit. The earlier exit retains its full forward count.
    #[test]
    fn later_body_point_keeps_each_earlier_exit() {
        for exit in [
            "do break end",
            "do continue end",
            "if not done then break end",
        ] {
            let mut count = Interrupts::default();
            for _ in 0..13 {
                assert!(!count.before("local x = 0", 1));
            }
            assert!(!count.before_at("while true do", 1, 100));
            assert!(!count.before("local first = 1", 1));
            assert!(!count.before(exit, 1));
            assert!(count.before("local later = 2", 1));
            assert_eq!(count.take_hoisted(), Some(100));
            assert!(!count.before("end", 1));
            assert!(!count.before("local after = 3", 1));
            assert_eq!(count.flow.count, 4);
        }
    }
}

#[cfg(test)]
mod later_tests {
    use super::Interrupts;
    #[test]
    fn later_statement_moves_point_and_reduces_exit_counts() {
        let mut c = Interrupts::default();
        for _ in 0..12 {
            assert!(!c.before("local x = 0", 1));
        }
        assert!(!c.before_at("while true do", 1, 100));
        assert!(!c.before("local first = 0", 1));
        assert!(!c.before("if done then break end", 1));
        assert!(c.before("local later = 0", 3));
        assert_eq!(c.take_hoisted(), Some(100));
        assert!(!c.before("end", 1));
        assert_eq!(c.flow.count, 5);
    }
}

#[cfg(test)]
mod reset_tests {
    use super::Interrupts;
    #[test]
    fn explicit_store_weight_is_authoritative() {
        for (cost, text) in [
            (2, "__ulua_b0.__ulua_k0 = (x) :: number"),
            (8, "object.field = (x) :: number"),
        ] {
            let mut c = Interrupts::default();
            for _ in 0..16 / cost {
                assert!(!c.before_store_at(text, 1, 0, Some(cost)));
            }
            assert!(c.before("local next = 0", 1));
        }
    }
    #[test]
    fn element_and_spill_stores_keep_eight() {
        for target in ["__ulua_b0[1]", "__ulua_spill0.__ulua_b1"] {
            let mut c = Interrupts::default();
            for _ in 0..2 {
                assert!(!c.before(&format!("{target} = (x) :: number"), 1));
            }
            assert!(c.before("local next = 0", 1));
        }
    }
    #[test]
    fn resets_block_later_hoists_including_at_branch_joins() {
        for reset in ["__ulua_b0()", "return x", "do return x end", "point"] {
            for branch in [false, true] {
                let mut c = Interrupts::default();
                for _ in 0..10 {
                    c.before("local x = 0", 1);
                }
                c.before_at("while true do", 1, 100);
                c.before("local first = 0", 1);
                if branch {
                    c.before("if b then", 1);
                }
                if reset == "point" {
                    // The reduced body would not fit; this point must remain local.
                    assert!(c.before("local big = 0", 16));
                    assert_eq!(c.take_hoisted(), None);
                } else {
                    c.before(reset, 1);
                }
                if branch {
                    c.before("else", 1);
                    c.before("local other = 0", 1);
                    c.before("end", 1);
                }
                // Fill the remaining count without introducing another reset.
                while c.flow.count < 16 {
                    assert!(!c.before("local x = 0", 1));
                }
                assert!(c.before("local last = 0", 1));
                assert_eq!(c.take_hoisted(), None, "{reset}, branch={branch}");
            }
        }
    }
    #[test]
    fn nested_move_reduces_every_inner_frame_and_saved_exit() {
        let mut c = Interrupts::default();
        for _ in 0..10 {
            c.before("local x = 0", 1);
        }
        c.before_at("for i = 1, 2 do", 1, 100);
        c.before_at("for j = 1, 2 do", 1, 200);
        c.before("if b then", 1);
        c.before("if done then break end", 1);
        assert!(c.before("local later = 0", 3));
        assert_eq!(c.take_hoisted(), Some(200));
        assert_eq!(c.flow.count, 6);
        assert_eq!(c.frames[1].entry.count, 1);
        assert_eq!(c.frames[1].largest.count, 3);
        assert_eq!(c.frames[2].entry.count, 2);
        c.before("else", 1);
        assert_eq!(c.flow.count, 2);
        c.before("end", 1);
        c.before("end", 1);
        assert_eq!(c.flow.count, 6);
        c.before("end", 1);
        assert_eq!(c.flow.count, 11); // The outer zero-trip path retains its count.
    }
}

#[cfg(test)]
mod exit_tests {
    use super::Interrupts;

    #[test]
    fn moved_point_and_later_local_points_preserve_each_saved_exit() {
        for exit in [
            "do break end",
            "do continue end",
            "if not done then break end",
        ] {
            let mut c = Interrupts::default();
            for _ in 0..13 {
                c.before("local x = 0", 1);
            }
            c.before_at("while true do", 1, 100);
            c.before("do", 1);
            c.before("local first = 0", 1);
            c.before(exit, 1);
            assert!(c.before("local later = 0", 1));
            assert_eq!(c.take_hoisted(), Some(100));
            assert_eq!(c.frames[1].entry.count, 0);
            assert!(c.before("local large = 0", 16));
            assert_eq!(c.take_hoisted(), None);
            assert!(c.before("local tail = 0", 1));
            c.before("end", 1);
            c.before("end", 1);
            assert_eq!(c.flow.count, 2, "{exit}");
        }
    }
}
