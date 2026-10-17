//! Bounded integer facts select the remainder form before emission.
mod components;
#[cfg(test)]
mod tests;
mod walk;
use crate::Diagnostic;
use crate::ir::{Expr, ExprKind as E, Module, NATIVE_REMAINDER, Statement as S};
use std::collections::{HashMap, HashSet};
use walk::Visitor;
const MAX: u64 = 1 << 53;
const FACT_ROUNDS: usize = 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fact {
    Unset,
    None,
    Integer(u64, u64),
}
impl Fact {
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::None, _) | (_, Self::None) => Self::None,
            (Self::Unset, b) | (b, Self::Unset) => b,
            (Self::Integer(a, b), Self::Integer(c, d)) => Self::Integer(a.min(c), b.max(d)),
        }
    }
}
#[derive(Clone, Copy)]
enum Node {
    None,
    Literal(u64),
    Read(usize),
    Binary(&'static str, usize, usize),
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Site {
    Expression(usize),
    Assignment(usize),
}
struct Write {
    binding: usize,
    value: usize,
    start: usize,
}
#[derive(Default)]
struct Graph {
    nodes: Vec<Node>,
    writes: Vec<Write>,
    bindings: Vec<usize>,
    sites: Vec<(usize, Site, u32)>,
}
struct Build<'a> {
    source: &'a [u8],
    graph: &'a mut Graph,
    indices: HashMap<usize, usize>,
    starts: HashMap<usize, usize>,
    stack: Vec<usize>,
    owners: &'a [usize],
    eligible: &'a mut [bool],
}
fn address<T>(value: &T) -> usize {
    std::ptr::from_ref(value) as usize
}
// Decode the checked literal's IEEE representation with integer operations only.
// The parser already resolves the source literal to its language number value.
fn integer(value: f64) -> Option<u64> {
    let bits = value.to_bits();
    if bits == 0 {
        return Some(0);
    }
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023;
    if bits >> 63 != 0 || !(0..=53).contains(&exponent) {
        return None;
    }
    let mantissa = (bits & ((1 << 52) - 1)) | (1 << 52);
    let result = if exponent >= 52 {
        mantissa << (exponent - 52)
    } else {
        let shift = 52 - exponent;
        if mantissa & ((1 << shift) - 1) != 0 {
            return None;
        }
        mantissa >> shift
    };
    (result <= MAX).then_some(result)
}
impl Build<'_> {
    fn push(&mut self, node: Node) -> usize {
        let id = self.graph.nodes.len();
        self.graph.nodes.push(node);
        id
    }
    fn lookup(&self, map: &HashMap<usize, usize>, e: &Expr) -> Result<usize, Diagnostic> {
        map.get(&address(e)).copied().ok_or_else(|| {
            Diagnostic::at(
                self.source,
                e.start as usize,
                "internal",
                "Missing remainder expression node",
            )
        })
    }
    fn index(&self, e: &Expr) -> Result<usize, Diagnostic> {
        self.lookup(&self.indices, e)
    }
    fn binary(&mut self, op: &'static str, a: usize, b: usize, key: Site, start: u32) -> usize {
        let id = self.push(Node::Binary(op, a, b));
        if op == "%" {
            self.graph.sites.push((id, key, start));
        }
        id
    }
}
impl Visitor for Build<'_> {
    fn enter_expression(&mut self) {
        self.stack.push(self.graph.nodes.len());
    }
    fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
        let id = match &e.kind {
            E::Number(n) => self.push(integer(*n).map_or(Node::None, Node::Literal)),
            E::Read(id) => self.push(Node::Read(*id)),
            E::Binary(op, a, b) => self.binary(
                op,
                self.index(a)?,
                self.index(b)?,
                Site::Expression(address(e)),
                e.start,
            ),
            _ => self.push(Node::None),
        };
        self.indices.insert(address(e), id);
        let start = self.stack.pop().ok_or_else(|| {
            Diagnostic::at(
                self.source,
                e.start as usize,
                "internal",
                "Missing remainder expression start",
            )
        })?;
        self.starts.insert(address(e), start);
        Ok(())
    }
    fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
        let key = address(s);
        match s {
            S::Declare(id, e) => {
                if self.owners[*id] != 0 {
                    self.eligible[*id] = true;
                    self.graph.bindings.push(*id);
                }
                self.graph.writes.push(Write {
                    binding: *id,
                    value: self.index(e)?,
                    start: self.lookup(&self.starts, e)?,
                });
            }
            S::Assign(id, op, e) => {
                let start = self.lookup(&self.starts, e)?;
                let value = self.index(e)?;
                let value = if let Some(op) = op {
                    let read = self.push(Node::Read(*id));
                    self.binary(op, read, value, Site::Assignment(key), e.start)
                } else {
                    value
                };
                if let S::Assign(id, _, _) = s {
                    self.graph.writes.push(Write {
                        binding: *id,
                        value,
                        start,
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }
}
fn binary(op: &str, a: Fact, b: Fact, increment: bool) -> Fact {
    let (Fact::Integer(al, ah), Fact::Integer(bl, bh)) = (a, b) else {
        return if a == Fact::None || b == Fact::None {
            Fact::None
        } else {
            Fact::Unset
        };
    };
    let bounds = match op {
        "+" if increment => Some(((al + bl).min(MAX), (ah + bh).min(MAX))),
        "+" if ah + bh <= MAX => Some((al + bl, ah + bh)),
        "-" if al >= bh => Some((al - bh, ah - bl)),
        "*" if u128::from(ah) * u128::from(bh) <= u128::from(MAX) => Some((al * bl, ah * bh)),
        "%" if bl >= 1 => Some((0, ah.min(bh - 1))),
        _ => None,
    };
    bounds.map_or(Fact::None, |(lo, hi)| Fact::Integer(lo, hi))
}
fn evaluate_range(
    graph: &Graph,
    bindings: &[Fact],
    values: &mut [Fact],
    range: std::ops::RangeInclusive<usize>,
) {
    for index in range {
        values[index] = match graph.nodes[index] {
            Node::None => Fact::None,
            Node::Literal(n) => Fact::Integer(n, n),
            Node::Read(id) => bindings[id],
            Node::Binary(op, a, b) => binary(
                op,
                values[a],
                values[b],
                matches!(graph.nodes[a], Node::Literal(1))
                    || matches!(graph.nodes[b], Node::Literal(1)),
            ),
        };
    }
}
fn evaluate(graph: &Graph, bindings: &[Fact], values: &mut Vec<Fact>) {
    values.resize(graph.nodes.len(), Fact::Unset);
    if !graph.nodes.is_empty() {
        evaluate_range(graph, bindings, values, 0..=graph.nodes.len() - 1);
    }
}
// Seven fixed radix passes sort the bounded thresholds in linear time.
fn thresholds(values: &mut Vec<(usize, u64)>) {
    let mut scratch = vec![(0, 0); values.len()];
    for shift in (0..56).step_by(8) {
        let mut ends = [0usize; 256];
        for n in values.iter() {
            ends[((n.1 >> shift) & 255) as usize] += 1;
        }
        let mut sum = 0;
        for end in &mut ends {
            sum += *end;
            *end = sum;
        }
        for n in values.iter().rev() {
            let bucket = ((n.1 >> shift) & 255) as usize;
            ends[bucket] -= 1;
            scratch[ends[bucket]] = *n;
        }
        std::mem::swap(values, &mut scratch);
    }
}
struct Mark<'a>(&'a HashSet<Site>);
impl Visitor for Mark<'_> {
    fn expression(&mut self, e: &mut Expr) -> Result<(), Diagnostic> {
        if self.0.contains(&Site::Expression(address(e)))
            && let E::Binary(op, _, _) = &mut e.kind
        {
            *op = NATIVE_REMAINDER;
        }
        Ok(())
    }
    fn statement(&mut self, s: &mut S) -> Result<(), Diagnostic> {
        if self.0.contains(&Site::Assignment(address(s)))
            && let S::Assign(_, Some(op), _) = s
        {
            *op = NATIVE_REMAINDER;
        }
        Ok(())
    }
}
fn visit_module(module: &mut Module, visitor: &mut impl Visitor) -> Result<(), Diagnostic> {
    let walk = walk::Walk {
        source: &module.source,
        limit: module.depth,
    };
    walk.statements(&mut module.body, visitor, 0)?;
    for f in &mut module.functions {
        for (_, e) in &mut f.parameters {
            if let Some(e) = e {
                walk.expression(e, visitor, 0)?;
            }
        }
        walk.statements(&mut f.initializers, visitor, 0)?;
        walk.statements(&mut f.body, visitor, 0)?;
    }
    Ok(())
}
#[derive(Default)]
pub(crate) struct Report {
    pub(crate) rounds: usize,
    pub(crate) sites: Vec<u32>,
    pub(crate) components_at_limit: usize,
}
pub(crate) fn analyze(module: &mut Module) -> Result<Report, Diagnostic> {
    let walk = walk::Walk {
        source: &module.source,
        limit: module.depth,
    };
    let owners: Vec<_> = module.bindings.iter().map(|b| b.owner).collect();
    let mut roots = vec![0; owners.len()];
    for (id, &owner) in owners.iter().enumerate() {
        roots[id] = if owner == 0 { id + 1 } else { roots[owner - 1] };
    }
    let mut graphs: Vec<_> = (0..=owners.len()).map(|_| Graph::default()).collect();
    let mut eligible = vec![false; owners.len()];
    let mut build = Build {
        source: &module.source,
        graph: &mut graphs[0],
        indices: HashMap::new(),
        starts: HashMap::new(),
        stack: Vec::new(),
        owners: &owners,
        eligible: &mut eligible,
    };
    walk.statements(&mut module.body, &mut build, 0)?;
    for f in &mut module.functions {
        let graph = &mut graphs[roots[f.binding]];
        let mut build = Build {
            source: &module.source,
            graph,
            indices: HashMap::new(),
            starts: HashMap::new(),
            stack: Vec::new(),
            owners: &owners,
            eligible: &mut eligible,
        };
        for (_, e) in &mut f.parameters {
            if let Some(e) = e {
                walk.expression(e, &mut build, 0)?;
            }
        }
        walk.statements(&mut f.initializers, &mut build, 0)?;
        walk.statements(&mut f.body, &mut build, 0)?;
    }
    let mut bindings: Vec<_> = eligible
        .iter()
        .map(|e| if *e { Fact::Unset } else { Fact::None })
        .collect();
    let mut report = Report::default();
    let mut native = HashSet::new();
    let mut values = vec![];
    for graph in graphs.iter_mut().filter(|s| !s.nodes.is_empty()) {
        let (rounds, at_limit) = components::solve(graph, &mut bindings);
        report.rounds = report.rounds.max(rounds);
        report.components_at_limit += at_limit;
        evaluate(graph, &bindings, &mut values);
        for &(node, key, start) in &graph.sites {
            if matches!(values[node], Fact::Integer(..)) {
                native.insert(key);
                report.sites.push(start);
            }
        }
    }
    visit_module(module, &mut Mark(&native))?;
    Ok(report)
}
