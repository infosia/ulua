//! Count generated statements on each forward path of one Luau function.
use crate::INTERRUPT_STATEMENTS;

#[derive(Clone, Copy, Default)]
struct Flow {
    count: usize,
    after_store: bool,
}
impl Flow {
    fn join(self, other: Self) -> Self {
        Self {
            count: self.count.max(other.count),
            after_store: self.after_store || other.after_store,
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
        let cost = (base + value.matches(".__ulua_").count()).min(INTERRUPT_STATEMENTS);
        if !block && !repeat && !argument_function {
            if self.flow.count + cost > INTERRUPT_STATEMENTS || self.flow.after_store {
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
        if table_store(text) {
            // A table write can resize or hash. Charge it before the next execution statement.
            self.flow.after_store = true;
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
    (callee.starts_with("__ulua_") || callee == "error")
        && !matches!(callee, "__ulua_fmod" | "__ulua_tostring" | "__ulua_gsub")
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
