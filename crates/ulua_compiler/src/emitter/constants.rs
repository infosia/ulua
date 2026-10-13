//! C7 excludes function identity. Numeric constant arrows have no invocation state.
use super::*;

pub(super) struct NumericArrows {
    pub(super) names: HashMap<usize, String>,
    pub(super) declarations: String,
    pub(super) locals: usize,
}

fn returned_number(body: &[ir::Statement]) -> Option<f64> {
    let mut result = None;
    let mut work: Vec<_> = body.iter().collect();
    while let Some(s) = work.pop() {
        match s.form() {
            ir::Statement::Entry(ids) if ids.is_empty() => {}
            ir::Statement::Block(b) => work.extend(b),
            ir::Statement::Return(Some(e)) if result.is_none() => {
                result = match &e.kind {
                    ir::ExprKind::Number(n) => Some(*n),
                    ir::ExprKind::Unary("-", a) => match a.kind {
                        ir::ExprKind::Number(n) => Some(-n),
                        _ => return None,
                    },
                    _ => return None,
                };
            }
            _ => return None,
        }
    }
    result
}

pub(super) fn numeric_arrows(
    module: &ir::Module,
    types: &types::FunctionTypes,
) -> Result<NumericArrows, Diagnostic> {
    let mut groups = HashMap::<u64, Vec<usize>>::new();
    for f in &module.functions {
        if f.name.is_empty()
            && f.result == Type::Number
            && f.parameters.is_empty()
            && f.initializers.is_empty()
            && let Some(n) = returned_number(&f.body)
        {
            groups.entry(n.to_bits()).or_default().push(f.binding);
        }
    }
    let mut groups: Vec<_> = groups
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .collect();
    groups.sort_unstable_by_key(|(bits, ids)| (std::cmp::Reverse(ids.len()), *bits));
    // A fixed cache keeps generated locals bounded; other arrows retain the ordinary lowering.
    groups.truncate(32);
    let mut result = NumericArrows {
        names: HashMap::new(),
        declarations: String::new(),
        locals: groups.len(),
    };
    for (index, (bits, ids)) in groups.into_iter().enumerate() {
        let name = format!("__ulua_number_arrow{index}");
        let ty = module.bindings[ids[0]].ty.ok_or_else(|| {
            Diagnostic::at(
                &module.source,
                module.bindings[ids[0]].start as usize,
                "internal",
                "Missing constant arrow signature",
            )
        })?;
        let _ = writeln!(
            result.declarations,
            "local {name}: {} = function(): number return {} end",
            types.luau(&ty),
            number(f64::from_bits(bits))
        );
        for id in ids {
            result.names.insert(id, name.clone());
        }
    }
    Ok(result)
}
