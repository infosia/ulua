//! Protected blocks retain scalar completions until the finalizer resolves them.
use super::*;
use crate::ir::Statement as S;
impl Emitter<'_> {
    pub(super) fn crosses_protection(&self) -> bool {
        self.protected
            .last()
            .is_some_and(|frame| self.loops.len() <= frame.loop_base)
    }
    pub(super) fn jump_completion(&mut self, tag: u32) {
        self.line(format!("do return {tag}, nil, {} end", self.loops.len()));
    }
    fn protected_block(
        &mut self,
        block: &S,
        binding: Option<usize>,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        let parameter = binding
            .map(|id| format!("__ulua_caught{id}: unknown"))
            .unwrap_or_default();
        self.line(format!("    function({parameter}): (number, any, number)"));
        let previous = binding.map(|id| (id, self.copies.insert(id, format!("__ulua_caught{id}"))));
        let offset = self.statement_offset;
        let index = self.protected_slots.len();
        self.protected_slots.push(vec![]);
        self.protected.push(Protection {
            loop_base: self.loops.len(),
            locals: binding
                .map(|id| format!("__ulua_caught{id}"))
                .into_iter()
                .collect(),
            upvalues: Default::default(),
            slots: Default::default(),
        });
        self.indent += 1;
        // Each completion edge otherwise versions the owner's mutable temporary upvalues.
        // Local shadows give this closure its own temporary flow graph and preserve scalar returns.
        let slots = if let Some(context) = self.context {
            context.layouts[self.owner]
                .as_ref()
                .and_then(|layout| layout.protected_slots.get(index))
                .ok_or_else(|| self.limit("internal", "Missing protected temporary layout"))?
                .clone()
        } else {
            vec![]
        };
        for i in slots {
            let name = self.temp(i);
            let ty = self
                .slots
                .get(i)
                .ok_or_else(|| self.limit("internal", "Missing protected temporary slot"))?
                .ty;
            if let Some(frame) = self.protected.last_mut() {
                frame.locals.insert(name.clone());
            }
            self.line(format!(
                "local {name}: {0} = {1}",
                self.function_types.luau(&ty),
                initial(&ty)
            ));
        }
        // A protected closure has its own flow graph, like a source function.
        // Its captured writes do not create parent joins across successive calls.
        let parent_joins = std::mem::take(&mut self.joins);
        self.statement(block, depth)?;
        if !ir::abrupt(block, true) {
            self.line("return 0, nil, 0");
        }
        self.joins = parent_joins;
        self.indent -= 1;
        let frame = self
            .protected
            .pop()
            .ok_or_else(|| self.limit("internal", "Missing protected closure capture frame"))?;
        self.protected_slots[index] = frame.slots.into_iter().collect();
        if frame.upvalues.len() > crate::UPVALUE_BUDGET {
            return Err(Diagnostic::at(
                &self.module.source,
                offset,
                "limit:upvalues",
                "Protected closure upvalue limit exceeded",
            ));
        }
        if let Some((id, old)) = previous {
            if let Some(old) = old {
                self.copies.insert(id, old);
            } else {
                self.copies.remove(&id);
            }
        }
        self.line("end");
        Ok(())
    }
    pub(super) fn try_statement(
        &mut self,
        body: &S,
        catch: Option<&(Option<usize>, Box<S>)>,
        finalizer: Option<&S>,
        terminates: bool,
        ends_block: bool,
        depth: u32,
    ) -> Result<(), Diagnostic> {
        let breaks = ir::loop_break(body)
            || catch.is_some_and(|(_, c)| ir::loop_break(c))
            || finalizer.is_some_and(ir::loop_break);
        let continues = ir::loop_continue(body)
            || catch.is_some_and(|(_, c)| ir::loop_continue(c))
            || finalizer.is_some_and(ir::loop_continue);
        let before = self.generated_live;
        let returns = ir::returns(body)
            || catch.is_some_and(|(_, c)| ir::returns(c))
            || finalizer.is_some_and(ir::returns);
        let never = terminates && !returns;
        let dispatch = !never && (returns || breaks || continues);
        self.line("do");
        let names = if dispatch {
            [self.generated()?, self.generated()?, self.generated()?]
        } else {
            Default::default()
        };
        let [tag, value, target] = &names;
        self.capture("__ulua_try");
        // Only source exits need dispatch in the caller's flow graph.
        if dispatch {
            self.line(format!(
                "local {tag}: number, {value}: any, {target}: number = __ulua_try("
            ));
        } else if never {
            // The helper cannot complete this source form. Expose that fact to the analyzer.
            self.line("error((__ulua_try(");
        } else {
            self.line("__ulua_try(");
        }
        self.protected_block(body, None, depth)?;
        self.line(",");
        if let Some((id, catch)) = catch {
            self.protected_block(catch, *id, depth)?;
        } else {
            self.line("    nil");
        }
        self.line(",");
        if let Some(finalizer) = finalizer {
            self.protected_block(finalizer, None, depth)?;
        } else {
            self.line("    nil");
        }
        self.line(if never { ")), 0)" } else { ")" });
        if dispatch && !self.protected.is_empty() {
            if terminates || (ends_block && self.crosses_protection()) {
                self.line(format!("do return {tag}, {value}, {target} end"));
            } else if returns {
                self.line(format!(
                    "if {tag} == 1 then return {tag}, {value}, {target} end"
                ));
            }
        } else if returns && self.owner != 0 {
            let result = self.module.bindings[self.owner - 1]
                .ty
                .and_then(|ty| self.module.types.function(ty))
                .map(|f| f.result)
                .ok_or_else(|| self.limit("internal", "Missing return signature"))?;
            if result == Type::Void {
                if terminates {
                    self.line("do return end");
                } else {
                    self.line(format!("if {tag} == 1 then return end"));
                }
            } else {
                // Tag 1 holds a checked return operand. Jumps retain nil and use their own dispatch.
                self.line(format!(
                    "{} return {value} end",
                    if terminates {
                        "do".to_owned()
                    } else {
                        format!("if {tag} == 1 then")
                    }
                ));
            }
        }
        if (breaks || continues)
            && !terminates
            && !ends_block
            && let Some(frame) = self.protected.last()
        {
            let base = frame.loop_base;
            self.line(format!("if ({tag} == 3 or {tag} == 4) and {target} <= {base} then return {tag}, {value}, {target} end"));
        }
        // The remaining jump belongs to a loop within this protected function.
        if !terminates && !self.loops.is_empty() && !self.crosses_protection() {
            if breaks {
                self.line(format!("if {tag} == 3 then"));
                self.statement(&S::Break, depth)?;
                self.line("end");
            }
            if continues {
                self.line(format!("if {tag} == 4 then"));
                self.statement(&S::Continue, depth)?;
                self.line("end");
            }
        }
        self.line("end");
        self.generated_live = before;
        Ok(())
    }
}
