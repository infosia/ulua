//! C7 literal-set formulas preserve const dependencies until parameter assignments are known.
use super::literal_sets::{Set, Sets};
use super::*;
use crate::ir::{Expr, ExprKind as E};

pub(super) const WIDE: usize = 0;
pub(super) const NARROWABLE: usize = 1;

#[derive(Hash, PartialEq, Eq)]
enum Literal {
    Number(u64),
    Boolean(bool),
    String(String),
}
#[derive(Clone, Copy)]
enum Fact {
    Wide,
    Narrowable,
    Finite(Set),
    Binding(usize),
    Not(usize),
    Conditional(usize, usize, usize),
    Logical(bool, usize, usize),
}

pub(super) struct Equality {
    pub(super) bindings: Vec<usize>,
    expressions: HashMap<u32, usize>,
    facts: Vec<Fact>,
    literals: HashMap<Literal, u32>,
    sets: Sets,
    checks: Vec<(Span, usize, usize)>,
    boolean_sets: [Set; 2],
}
impl Equality {
    pub(super) fn new() -> Self {
        let mut equality = Self {
            bindings: vec![],
            expressions: HashMap::new(),
            facts: vec![Fact::Wide, Fact::Narrowable],
            literals: HashMap::new(),
            sets: Sets::new(),
            checks: vec![],
            boolean_sets: [0; 2],
        };
        for value in [false, true] {
            let set = equality.literal(Literal::Boolean(value));
            equality.boolean_sets[usize::from(value)] = set;
        }
        equality
    }
    fn literal(&mut self, literal: Literal) -> Set {
        // The source ceiling bounds literal identities below u32::MAX.
        let next = self.literals.len() as u32;
        let id = *self.literals.entry(literal).or_insert(next);
        self.sets.singleton(id)
    }
    pub(super) fn expression(&self, value: &Expr) -> Option<usize> {
        self.expressions.get(&value.start).copied()
    }
    pub(super) fn check(&mut self, span: Span, left: usize, right: usize) {
        self.checks.push((span, left, right));
    }
}
impl Checker<'_> {
    pub(super) fn expression_fact(&self, value: &Expr) -> Result<usize, Diagnostic> {
        self.equality.expression(value).ok_or_else(|| {
            Diagnostic::at(
                self.source,
                value.start as usize,
                "internal",
                "Missing expression literal set",
            )
        })
    }
    pub(super) fn record_fact(
        &mut self,
        source: &Expression<'_>,
        value: &Expr,
    ) -> Result<(), Diagnostic> {
        let fact = match source {
            Expression::NumericLiteral(n) => {
                Fact::Finite(self.equality.literal(Literal::Number(number_key(n.value))))
            }
            Expression::BooleanLiteral(b) => {
                Fact::Finite(self.equality.boolean_sets[usize::from(b.value)])
            }
            Expression::StringLiteral(s) => Fact::Finite(
                self.equality
                    .literal(Literal::String(s.value.as_str().into())),
            ),
            Expression::TemplateLiteral(t) if t.expressions.is_empty() => {
                if let E::Template(parts) = &value.kind
                    && let Some(Expr {
                        kind: E::String(text),
                        ..
                    }) = parts.first()
                {
                    Fact::Finite(self.equality.literal(Literal::String(text.clone())))
                } else {
                    return Err(self.error(
                        source.span(),
                        "internal",
                        "Missing checked template text",
                    ));
                }
            }
            Expression::UnaryExpression(u) if u.operator.as_str() == "-" => {
                let mut argument = &u.argument;
                while let Expression::ParenthesizedExpression(p) = argument {
                    argument = &p.expression;
                }
                if let Expression::NumericLiteral(n) = argument {
                    Fact::Finite(self.equality.literal(Literal::Number(number_key(-n.value))))
                } else {
                    Fact::Wide
                }
            }
            _ => match &value.kind {
                E::Read(id) => Fact::Binding(*id),
                E::Unary("not", e) => Fact::Not(self.expression_fact(e)?),
                E::Conditional(c, a, b) => Fact::Conditional(
                    self.expression_fact(c)?,
                    self.expression_fact(a)?,
                    self.expression_fact(b)?,
                ),
                E::Logical(and, a, b) => {
                    Fact::Logical(*and, self.expression_fact(a)?, self.expression_fact(b)?)
                }
                _ => Fact::Wide,
            },
        };
        let id = self.equality.facts.len();
        self.equality.facts.push(fact);
        self.equality.expressions.insert(value.start, id);
        Ok(())
    }
    pub(super) fn check_equalities(&mut self) -> Result<(), Diagnostic> {
        let mut values = Vec::with_capacity(self.equality.facts.len());
        // Every binding dependency precedes its read (C10). This loop handles long const chains without recursion.
        for fact in self.equality.facts.iter().copied() {
            let get = |id| {
                values.get(id).copied().ok_or_else(|| {
                    self.error(
                        Span::default(),
                        "internal",
                        "Invalid literal-set dependency",
                    )
                })
            };
            let value = match fact {
                Fact::Binding(id) => get(self.equality.bindings[id])?,
                Fact::Not(id) => match get(id)? {
                    Fact::Finite(set) => {
                        let mut negated = 0;
                        for boolean in [false, true] {
                            if self
                                .equality
                                .sets
                                .overlap(set, self.equality.boolean_sets[usize::from(boolean)])
                            {
                                negated = self.equality.sets.union(
                                    negated,
                                    self.equality.boolean_sets[usize::from(!boolean)],
                                );
                            }
                        }
                        Fact::Finite(negated)
                    }
                    value => value,
                },
                Fact::Conditional(c, a, b) => {
                    let c = get(c)?;
                    let a = get(a)?;
                    let b = get(b)?;
                    if matches!(c, Fact::Narrowable)
                        || matches!(a, Fact::Narrowable)
                        || matches!(b, Fact::Narrowable)
                    {
                        Fact::Narrowable
                    } else if let (Fact::Finite(a), Fact::Finite(b)) = (a, b) {
                        Fact::Finite(self.equality.sets.union(a, b))
                    } else {
                        Fact::Wide
                    }
                }
                Fact::Logical(and, a, b) => {
                    let a = get(a)?;
                    let b = get(b)?;
                    if matches!(a, Fact::Narrowable) || matches!(b, Fact::Narrowable) {
                        Fact::Narrowable
                    } else {
                        let short = self.equality.boolean_sets[usize::from(!and)];
                        let enter = self.equality.boolean_sets[usize::from(and)];
                        let (can_short, can_enter) = match a {
                            Fact::Finite(a) => (
                                self.equality.sets.overlap(a, short),
                                self.equality.sets.overlap(a, enter),
                            ),
                            _ => (true, true),
                        };
                        let result = if can_enter { b } else { Fact::Finite(0) };
                        if let Fact::Finite(set) = result {
                            Fact::Finite(if can_short {
                                self.equality.sets.union(set, short)
                            } else {
                                set
                            })
                        } else {
                            result
                        }
                    }
                }
                value => value,
            };
            values.push(value);
        }
        for &(span, left, right) in &self.equality.checks {
            let left = values[left];
            let right = values[right];
            let rejected = match (left, right) {
                (Fact::Wide, _) | (_, Fact::Wide) => false,
                (Fact::Narrowable, _) | (_, Fact::Narrowable) => true,
                (Fact::Finite(a), Fact::Finite(b)) => !self.equality.sets.overlap(a, b),
                _ => return Err(self.error(span, "internal", "Unresolved literal-set formula")),
            };
            if rejected {
                return Err(self.error(
                    span,
                    "C7",
                    "Equality literal sets do not overlap or depend on narrowing",
                ));
            }
        }
        Ok(())
    }
}
fn number_key(number: f64) -> u64 {
    if number == 0.0 { 0 } else { number.to_bits() }
}
