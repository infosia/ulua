//! Bounded integer facts select the remainder form before emission.
mod components;
#[cfg(test)]
mod tests;
mod walk;
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
struct System {
    nodes: Vec<Node>,
    writes: Vec<Write>,
    bindings: Vec<usize>,
    sites: Vec<(usize, Site, u32)>,
    thresholds: Vec<u64>,
}
struct Build<'a> {
    system: &'a mut System,
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
        let id = self.system.nodes.len();
        self.system.nodes.push(node);
        id
    }
    fn index(&self, e: &Expr) -> usize {
        self.indices[&address(e)]
    }
    fn binary(&mut self, op: &'static str, a: usize, b: usize, key: Site, start: u32) -> usize {
        let id = self.push(Node::Binary(op, a, b));
        if op == "%" {
            self.system.sites.push((id, key, start));
            if let Node::Literal(b) = self.system.nodes[b]
                && b >= 1
            {
                self.system.thresholds.push(b - 1);
            }
        }
        id
    }
}
impl Visitor for Build<'_> {
    fn enter_expression(&mut self) {
        self.stack.push(self.system.nodes.len());
    }
    fn expression(&mut self, e: &mut Expr) {
        let id = match &e.kind {
            E::Number(n) => self.push(integer(*n).map_or(Node::None, Node::Literal)),
            E::Read(id) => self.push(Node::Read(*id)),
            E::Binary(op, a, b) => self.binary(
                op,
                self.index(a),
                self.index(b),
                Site::Expression(address(e)),
                e.start,
            ),
            _ => self.push(Node::None),
        };
        self.indices.insert(address(e), id);
        self.starts
            .insert(address(e), self.stack.pop().unwrap_or(id));
    }
    fn statement(&mut self, s: &mut S) {
        let key = address(s);
        match s {
            S::Declare(id, e) => {
                if self.owners[*id] != 0 {
                    self.eligible[*id] = true;
                    self.system.bindings.push(*id);
                }
                self.system.writes.push(Write {
                    binding: *id,
                    value: self.index(e),
                    start: self.starts[&address(e)],
                });
            }
            S::Assign(id, op, e) => {
                let start = self.starts[&address(e)];
                let value = self.index(e);
                let value = if let Some(op) = op {
                    let read = self.push(Node::Read(*id));
                    self.binary(op, read, value, Site::Assignment(key), e.start)
                } else {
                    value
                };
                if let S::Assign(id, _, _) = s {
                    self.system.writes.push(Write {
                        binding: *id,
                        value,
                        start,
                    });
                }
            }
            _ => {}
        }
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
    system: &System,
    bindings: &[Fact],
    values: &mut [Fact],
    range: std::ops::RangeInclusive<usize>,
) {
    for index in range {
        values[index] = match system.nodes[index] {
            Node::None => Fact::None,
            Node::Literal(n) => Fact::Integer(n, n),
            Node::Read(id) => bindings[id],
            Node::Binary(op, a, b) => binary(
                op,
                values[a],
                values[b],
                matches!(system.nodes[a], Node::Literal(1))
                    || matches!(system.nodes[b], Node::Literal(1)),
            ),
        };
    }
}
fn evaluate(system: &System, bindings: &[Fact], values: &mut Vec<Fact>) {
    values.resize(system.nodes.len(), Fact::Unset);
    if !system.nodes.is_empty() {
        evaluate_range(system, bindings, values, 0..=system.nodes.len() - 1);
    }
}
// Seven fixed radix passes sort the bounded thresholds in linear time.
fn thresholds(values: &mut Vec<u64>) {
    values.push(MAX);
    let mut scratch = vec![0; values.len()];
    for shift in (0..56).step_by(8) {
        let mut ends = [0usize; 256];
        for n in values.iter() {
            ends[((n >> shift) & 255) as usize] += 1;
        }
        let mut sum = 0;
        for end in &mut ends {
            sum += *end;
            *end = sum;
        }
        for n in values.iter().rev() {
            let bucket = ((n >> shift) & 255) as usize;
            ends[bucket] -= 1;
            scratch[ends[bucket]] = *n;
        }
        std::mem::swap(values, &mut scratch);
    }
    values.dedup();
}
struct Mark<'a>(&'a HashSet<Site>);
impl Visitor for Mark<'_> {
    fn expression(&mut self, e: &mut Expr) {
        if self.0.contains(&Site::Expression(address(e)))
            && let E::Binary(op, _, _) = &mut e.kind
        {
            *op = NATIVE_REMAINDER;
        }
    }
    fn statement(&mut self, s: &mut S) {
        if self.0.contains(&Site::Assignment(address(s)))
            && let S::Assign(_, Some(op), _) = s
        {
            *op = NATIVE_REMAINDER;
        }
    }
}
fn visit_module(module: &mut Module, visitor: &mut impl Visitor) {
    walk::statements(&mut module.body, visitor);
    for f in &mut module.functions {
        for (_, e) in &mut f.parameters {
            if let Some(e) = e {
                walk::expression(e, visitor);
            }
        }
        walk::statements(&mut f.initializers, visitor);
        walk::statements(&mut f.body, visitor);
    }
}
#[derive(Default)]
pub(crate) struct Report {
    pub(crate) rounds: usize,
    pub(crate) sites: Vec<u32>,
    pub(crate) components_at_limit: usize,
}
pub(crate) fn analyze(module: &mut Module) -> Report {
    let owners: Vec<_> = module.bindings.iter().map(|b| b.owner).collect();
    let mut roots = vec![0; owners.len()];
    for (id, &owner) in owners.iter().enumerate() {
        roots[id] = if owner == 0 { id + 1 } else { roots[owner - 1] };
    }
    let mut systems: Vec<_> = (0..=owners.len()).map(|_| System::default()).collect();
    let mut eligible = vec![false; owners.len()];
    let mut build = Build {
        system: &mut systems[0],
        indices: HashMap::new(),
        starts: HashMap::new(),
        stack: Vec::new(),
        owners: &owners,
        eligible: &mut eligible,
    };
    walk::statements(&mut module.body, &mut build);
    for f in &mut module.functions {
        let system = &mut systems[roots[f.binding]];
        let mut build = Build {
            system,
            indices: HashMap::new(),
            starts: HashMap::new(),
            stack: Vec::new(),
            owners: &owners,
            eligible: &mut eligible,
        };
        for (_, e) in &mut f.parameters {
            if let Some(e) = e {
                walk::expression(e, &mut build);
            }
        }
        walk::statements(&mut f.initializers, &mut build);
        walk::statements(&mut f.body, &mut build);
    }
    let mut bindings: Vec<_> = eligible
        .iter()
        .map(|e| if *e { Fact::Unset } else { Fact::None })
        .collect();
    let mut report = Report::default();
    let mut native = HashSet::new();
    let mut values = vec![];
    for system in systems.iter_mut().filter(|s| !s.nodes.is_empty()) {
        let (rounds, at_limit) = components::solve(system, &mut bindings);
        report.rounds = report.rounds.max(rounds);
        report.components_at_limit += at_limit;
        evaluate(system, &bindings, &mut values);
        for &(node, key, start) in &system.sites {
            if matches!(values[node], Fact::Integer(..)) {
                native.insert(key);
                report.sites.push(start);
            }
        }
    }
    visit_module(module, &mut Mark(&native));
    report
}
