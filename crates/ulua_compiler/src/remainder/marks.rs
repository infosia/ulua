//! Greatest fixed point: each node loses its mark once, propagating along reverse edges.
//! Expression nodes keep write dependencies linear even for shared binding reads.
use super::*;
use crate::ir::IndexForm;

struct Build<'a> {
    source: &'a [u8],
    owners: Vec<usize>,
    parameters: &'a parameters::Parameters,
    facts: &'a HashMap<usize, usize>,
    values: &'a [Fact],
    indices: HashMap<usize, usize>,
    eligible: Vec<bool>,
    marked: Vec<bool>,
    users: Vec<Vec<usize>>,
}
impl Build<'_> {
    fn node(&mut self, marked: bool) -> usize {
        let id = self.marked.len();
        self.marked.push(marked);
        self.users.push(vec![]);
        id
    }
    fn edge(&mut self, dependency: usize, user: usize) {
        self.users[dependency].push(user);
    }
    fn index(&self, e: &Expr) -> Result<usize, Diagnostic> {
        self.indices.get(&address(e)).copied().ok_or_else(|| {
            Diagnostic::at(
                self.source,
                e.start as usize,
                "internal",
                "Missing integral expression node",
            )
        })
    }
    fn binary(&mut self, op: &str, a: usize, b: usize) -> usize {
        let marked = matches!(op, "+" | "-" | "*" | "%" | NATIVE_REMAINDER);
        let id = self.node(marked);
        if marked {
            self.edge(a, id);
            self.edge(b, id);
        }
        id
    }
}
impl Visitor for Build<'_> {
    fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
        let fact = self
            .facts
            .get(&address(e))
            .and_then(|id| self.values.get(*id))
            .is_some_and(|f| matches!(f, Fact::Integer(..)));
        let id = if fact {
            self.node(true)
        } else {
            match &e.kind {
                E::Number(n) => self.node(!n.is_finite() || n.fract() == 0.0),
                E::Length(_) => self.node(true),
                E::Read(binding) => {
                    let id = self.node(true);
                    self.edge(*binding, id);
                    id
                }
                E::Unary("-", a) => {
                    let id = self.node(true);
                    self.edge(self.index(a)?, id);
                    id
                }
                E::Binary(op, a, b) => self.binary(op, self.index(a)?, self.index(b)?),
                E::Call(callee, args) => match callee.kind {
                    E::Builtin(
                        "__ulua_floor" | "__ulua_ceil" | "__ulua_math_trunc" | "__ulua_math_round",
                    ) => self.node(true),
                    E::Builtin("__ulua_math_abs") if args.len() == 1 => {
                        let id = self.node(true);
                        self.edge(self.index(&args[0])?, id);
                        id
                    }
                    _ => self.node(false),
                },
                _ => self.node(false),
            }
        };
        self.indices.insert(address(e), id);
        // Parameter-call edges are required even when the call expression has a fact.
        if let E::Call(callee, args) = &e.kind
            && let E::Read(binding) = callee.kind
            && let Some(parameters) = self.parameters.0.get(&binding)
        {
            for (parameter, argument) in parameters.iter().zip(args) {
                if let Some(parameter) = parameter {
                    self.edge(self.index(argument)?, *parameter);
                }
            }
        }
        Ok(())
    }
    fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
        match s {
            S::Declare(binding, e) => {
                // A nonzero owner copies the function-local binding eligibility of item 25.
                self.eligible[*binding] = self.owners[*binding] != 0;
                self.edge(self.index(e)?, *binding);
            }
            S::Assign(binding, op, e) => {
                let value = self.index(e)?;
                let value = if let Some(op) = op {
                    self.binary(op, *binding, value)
                } else {
                    value
                };
                self.edge(value, *binding);
            }
            _ => {}
        }
        Ok(())
    }
}
struct Mark<'a> {
    source: &'a [u8],
    indices: &'a HashMap<usize, usize>,
    marked: &'a [bool],
}
impl Mark<'_> {
    fn index(&self, e: &mut Expr) -> Result<(), Diagnostic> {
        let Some(id) = self.indices.get(&address(e)) else {
            return Err(Diagnostic::at(
                self.source,
                e.start as usize,
                "internal",
                "Missing integral index node",
            ));
        };
        if e.index_form == IndexForm::Checked && self.marked[*id] {
            e.index_form = IndexForm::Integral;
        }
        Ok(())
    }
}
impl Visitor for Mark<'_> {
    fn expression(&mut self, _: &mut Expr) -> Result<(), Diagnostic> {
        Ok(())
    }
    fn index_expression(&mut self, e: &mut Expr, receiver: bool) -> Result<(), Diagnostic> {
        if !receiver && let E::Index(_, i) = &mut e.kind {
            self.index(i)?;
        }
        Ok(())
    }
    fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
        if let S::Element(_, i, _, _) = s {
            self.index(i)?;
        }
        Ok(())
    }
}
pub(super) fn analyze(
    module: &mut Module,
    parameters: &parameters::Parameters,
    facts: &HashMap<usize, usize>,
    values: &[Fact],
) -> Result<(), Diagnostic> {
    let count = module.bindings.len();
    let source = module.source.clone();
    let mut build = Build {
        source: &source,
        owners: module.bindings.iter().map(|b| b.owner).collect(),
        parameters,
        facts,
        values,
        indices: HashMap::new(),
        eligible: vec![false; count],
        // Binding ids share the expression-node index space, occupying its first count nodes.
        marked: vec![true; count],
        users: vec![vec![]; count],
    };
    for targets in parameters.0.values() {
        for &id in targets.iter().flatten() {
            build.eligible[id] = true;
        }
    }
    visit_module(module, &mut build)?;
    for (id, eligible) in build.eligible.iter().enumerate() {
        build.marked[id] = *eligible;
    }
    let mut pending: Vec<_> = build
        .marked
        .iter()
        .enumerate()
        .filter_map(|(id, marked)| (!marked).then_some(id))
        .collect();
    while let Some(id) = pending.pop() {
        for &user in &build.users[id] {
            if build.marked[user] {
                build.marked[user] = false;
                pending.push(user);
            }
        }
    }
    visit_module(
        module,
        &mut Mark {
            source: &source,
            indices: &build.indices,
            marked: &build.marked,
        },
    )
}
