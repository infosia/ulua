//! Protected blocks retain scalar completions until the finalizer resolves them.
use super::*;
use crate::ir::Statement as S;
impl Emitter<'_> {
    pub(super) fn crosses_protection(&self) -> bool {
        self.protected
            .last()
            .is_some_and(|base| self.loops.len() <= *base)
    }
    pub(super) fn jump_completion(&mut self, tag: u32) {
        self.line(format!("do return {tag}, nil, {} end", self.loops.len()));
    }
    fn protected_block(
        &mut self,
        block: &S,
        binding: Option<(usize, &str)>,
        names: &[String; 4],
        depth: u32,
    ) -> Result<(), Diagnostic> {
        let [ok, tag, value, target] = names;
        let parameter = binding
            .map(|(id, _)| format!("__ulua_caught{id}: unknown"))
            .unwrap_or_default();
        self.line(format!("local {ok}, {tag}, {value}, {target} = pcall(function({parameter}): (number, any, number)"));
        let previous =
            binding.map(|(id, _)| (id, self.copies.insert(id, format!("__ulua_caught{id}"))));
        self.protected.push(self.loops.len());
        self.indent += 1;
        self.joins.enter();
        self.statement(block, depth)?;
        if !ir::abrupt(block, true) {
            self.line("return 0, nil, 0");
        }
        self.joins.leave();
        self.indent -= 1;
        self.protected.pop();
        if let Some((id, old)) = previous {
            if let Some(old) = old {
                self.copies.insert(id, old);
            } else {
                self.copies.remove(&id);
            }
        }
        self.line(binding.map_or_else(|| "end)".into(), |(_, value)| format!("end, {value})")));
        self.capture("__ulua_instance");
        self.line(format!("if not {ok} then"));
        self.line(format!(
            "if not __ulua_instance({tag}, \"Error\") then error({tag}, 0) end"
        ));
        self.line(format!("{value} = {tag}; {tag} = 2; {target} = 0"));
        self.line("end");
        Ok(())
    }
    fn completion_names(&mut self) -> Result<[String; 4], Diagnostic> {
        Ok([
            self.generated()?,
            self.generated()?,
            self.generated()?,
            self.generated()?,
        ])
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
        let before = self.generated_live;
        self.line("do");
        let names = self.completion_names()?;
        self.protected_block(body, None, &names, depth)?;
        let [_, tag, value, target] = &names;
        if let Some((id, catch)) = catch {
            self.line(format!("if {tag} == 2 then"));
            let caught = self.completion_names()?;
            self.protected_block(catch, id.map(|id| (id, value.as_str())), &caught, depth)?;
            self.line(format!(
                "{tag}, {value}, {target} = {}, {}, {}",
                caught[1], caught[2], caught[3]
            ));
            self.line("end");
            self.generated_live -= 4;
        }
        if let Some(finalizer) = finalizer {
            self.line("do");
            let final_names = self.completion_names()?;
            self.protected_block(finalizer, None, &final_names, depth)?;
            self.line(format!("if {} ~= 0 then", final_names[1]));
            self.line(format!(
                "{tag}, {value}, {target} = {}, {}, {}",
                final_names[1], final_names[2], final_names[3]
            ));
            self.line("end");
            self.line("end");
            self.generated_live -= 4;
        }
        self.line(format!("if {tag} == 2 then error({value}, 0) end"));
        if !self.protected.is_empty() {
            if terminates || (ends_block && self.crosses_protection()) {
                self.line(format!("do return {tag}, {value}, {target} end"));
            } else {
                self.line(format!(
                    "if {tag} == 1 then return {tag}, {value}, {target} end"
                ));
            }
        } else if self.owner != 0 {
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
                self.line(format!(
                    "{} return {value} :: {} end",
                    if terminates {
                        "do".to_owned()
                    } else {
                        format!("if {tag} == 1 then")
                    },
                    self.function_types.luau(&result)
                ));
            }
        }
        if !terminates
            && !ends_block
            && let Some(base) = self.protected.last()
        {
            self.line(format!("if ({tag} == 3 or {tag} == 4) and {target} <= {base} then return {tag}, {value}, {target} end"));
        }
        // The remaining jump belongs to a loop within this protected function.
        if !terminates && !self.loops.is_empty() && !self.crosses_protection() {
            self.line(format!("if {tag} == 3 then"));
            self.statement(&S::Break, depth)?;
            self.line("end");
            self.line(format!("if {tag} == 4 then"));
            self.statement(&S::Continue, depth)?;
            self.line("end");
        }
        self.line("end");
        self.generated_live = before;
        Ok(())
    }
}
