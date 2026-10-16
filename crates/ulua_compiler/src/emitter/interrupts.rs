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
}
struct Frame {
    entry: Flow,
    largest: Flow,
    kind: Kind,
    has_else: bool,
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
    pub(super) fn before(&mut self, text: &str) -> bool {
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
            .min(INTERRUPT_STATEMENTS);
        if !block && !repeat && !argument_function {
            if self.flow.count + cost > INTERRUPT_STATEMENTS {
                insert = true;
                self.flow = Flow::default();
            }
            self.flow.count += cost;
        }
        if function || block || repeat || branch || loop_head {
            self.frames.push(Frame {
                entry: self.flow,
                largest: Flow::default(),
                has_else: false,
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
                    assert!(!counter.before(&statement), "{statement}");
                }
                assert!(
                    counter.before("local next = 1"),
                    "{statement} erased the count"
                );
            }
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
