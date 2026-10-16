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
    hoisted: Option<usize>,
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
    pub(super) fn before(&mut self, text: &str, minimum_cost: usize) -> bool {
        self.before_at(text, minimum_cost, 0)
    }
    pub(super) fn take_hoisted(&mut self) -> Option<usize> {
        self.hoisted.take()
    }
    pub(super) fn checkpoint(&mut self) {
        self.flow = Flow::default();
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
    // Pending loop delimiters carry their output position and counted header work.
    fn hoist_head(&mut self, cost: usize) -> usize {
        let function = self
            .frames
            .iter()
            .rposition(|frame| matches!(frame.kind, Kind::Function))
            .map_or(0, |index| index + 1);
        let Some(index) =
            (function..self.frames.len()).find(|index| self.frames[*index].head.is_some())
        else {
            return 0;
        };
        let headers: usize = self.frames[index..]
            .iter()
            .filter_map(|frame| frame.head.map(|(_, cost)| cost))
            .sum();
        if headers + cost > INTERRUPT_STATEMENTS {
            return 0;
        }
        self.hoisted = self.frames[index].head.map(|(position, _)| position);
        let mut count = 0;
        for frame in &mut self.frames[index..] {
            count += frame.head.map_or(0, |(_, cost)| cost);
            frame.entry = Flow { count };
        }
        count
    }
    pub(super) fn before_at(&mut self, text: &str, minimum_cost: usize, position: usize) -> bool {
        self.hoisted = None;
        let text = text.trim();
        if text == "else" {
            if let Some(frame) = self.frames.last_mut() {
                frame.has_else = true;
                frame.largest = frame.largest.join(self.flow);
                self.flow = frame.entry;
            }
            return false;
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
            return false;
        }
        if text == "," || text == "nil" {
            return false;
        }
        if text == ")" || text == ")), 0)" {
            // Protected closure arguments are complete. The enclosing call always executes.
            self.flow = Flow::default();
            return false;
        }
        let argument_function = text.starts_with("function(");
        let function = argument_function || text.starts_with("local function ");
        let block = text == "do";
        let repeat = text == "repeat";
        let branch = text.starts_with("if ") && text.ends_with(" then");
        let loop_head = text.starts_with("while ") && text.ends_with(" do");
        let mut insert = false;
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
        let cost = (base + value.matches(".__ulua_").count())
            .max(weight)
            .max(minimum_cost)
            .min(INTERRUPT_STATEMENTS);
        // An unconditional while delimiter emits no forward instruction. Its condition is a separate statement.
        let executable = !block && !repeat && !argument_function && text != "while true do";
        if executable {
            if self.flow.count + cost > INTERRUPT_STATEMENTS {
                insert = true;
                self.flow.count = self.hoist_head(cost);
            }
            self.flow.count += cost;
            for frame in self.frames.iter_mut().rev() {
                if matches!(frame.kind, Kind::Function) {
                    break;
                }
                frame.head = None;
            }
        }
        if function || block || repeat || branch || loop_head {
            self.frames.push(Frame {
                entry: self.flow,
                largest: Flow::default(),
                has_else: false,
                head: (loop_head || repeat)
                    .then_some((position, if executable { cost } else { 0 })),
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
            self.flow = Flow::default();
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
        insert
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
            "__ulua_fmod" | "__ulua_floor" | "__ulua_tostring" | "__ulua_gsub" | "__ulua_print"
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
                "if v == nil or __ulua_floor(i) ~= i then __ulua_trap(1) end",
                3,
            ),
            (
                "if __ulua_floor(i) ~= i or (a[i + 1] == nil and i ~= #a) then __ulua_trap(1) end",
                4,
            ),
            ("__ulua_t0 = ((a == b)) :: boolean", 2),
            (
                "local __ulua_g0_1: boolean = __ulua_floor(i) ~= i or (a[i + 1] == nil and i ~= #a)",
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

    // Cost: 40 fixed classifications. No compiler, VM, or clock assertion.
    #[test]
    fn first_body_point_precedes_pending_loop_wrappers() {
        let mut count = Interrupts::default();
        for _ in 0..16 {
            assert!(!count.before("local x = 0", 1));
        }
        assert!(!count.before_at("while true do", 1, 100));
        assert!(!count.before_at("repeat", 1, 120));
        assert!(!count.before("do", 1));
        assert!(count.before("local first = 1", 1));
        assert_eq!(count.take_hoisted(), Some(100));
        for line in ["end", "until true", "end"] {
            assert!(!count.before(line, 1));
        }
        for _ in 0..15 {
            assert!(!count.before("local after = 0", 1));
        }
        assert!(count.before("local last = 0", 1));
    }

    // Cost: 20 fixed classifications per exit. The earlier exit retains its full forward count.
    #[test]
    fn later_body_point_keeps_each_earlier_exit() {
        for exit in [
            "do break end",
            "do continue end",
            "if not done then break end",
        ] {
            let mut count = Interrupts::default();
            for _ in 0..14 {
                assert!(!count.before("local x = 0", 1));
            }
            assert!(!count.before_at("while true do", 1, 100));
            assert!(!count.before("local first = 1", 1));
            assert!(!count.before(exit, 1));
            assert!(count.before("local later = 2", 1));
            assert_eq!(count.take_hoisted(), None);
            assert!(!count.before("end", 1));
            assert!(count.before("local after = 3", 1));
        }
    }
}
