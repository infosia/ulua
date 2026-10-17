//! Iterative component discovery and bounded, dependency-first fact rounds.
use super::{FACT_ROUNDS, Fact, Graph, Node, evaluate_range, thresholds};
use std::collections::HashMap;

// Edges point from a binding to the bindings that its writes read.
// Kosaraju's second pass returns dependents first, so reverse its components.
fn components(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut reverse = vec![vec![]; edges.len()];
    for (from, reads) in edges.iter().enumerate() {
        for &to in reads {
            reverse[to].push(from);
        }
    }
    let mut seen = vec![false; edges.len()];
    let mut order = Vec::with_capacity(edges.len());
    let mut stack = vec![];
    for root in 0..edges.len() {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        stack.push((root, 0));
        while let Some((id, cursor)) = stack.last_mut() {
            if let Some(&next) = edges[*id].get(*cursor) {
                *cursor += 1;
                if !seen[next] {
                    seen[next] = true;
                    stack.push((next, 0));
                }
            } else {
                order.push(*id);
                stack.pop();
            }
        }
    }
    seen.fill(false);
    let mut result = vec![];
    let mut pending = vec![];
    for root in order.into_iter().rev() {
        if seen[root] {
            continue;
        }
        let mut component = vec![];
        seen[root] = true;
        pending.push(root);
        while let Some(id) = pending.pop() {
            component.push(id);
            for &next in &reverse[id] {
                if !seen[next] {
                    seen[next] = true;
                    pending.push(next);
                }
            }
        }
        result.push(component);
    }
    result.reverse();
    result
}

pub(super) fn solve(graph: &mut Graph, bindings: &mut [Fact]) -> (usize, usize) {
    let positions: HashMap<_, _> = graph
        .bindings
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();
    // Sort once by value, then scatter. Seven radix passes cost O(writes + bindings).
    let mut tagged: Vec<_> = (0..positions.len()).map(|pos| (pos, super::MAX)).collect();
    for write in &graph.writes {
        if let Some(&pos) = positions.get(&write.binding) {
            let threshold = match graph.nodes[write.value] {
                Node::Literal(k) => Some(k),
                Node::Binary("%", _, b) => match graph.nodes[b] {
                    Node::Literal(b) if b >= 1 => Some(b - 1),
                    _ => None,
                },
                _ => None,
            };
            if let Some(value) = threshold {
                tagged.push((pos, value));
            }
        }
    }
    thresholds(&mut tagged);
    let mut thresholds = vec![vec![]; positions.len()];
    for (pos, value) in tagged {
        thresholds[pos].push(value);
    }
    for values in &mut thresholds {
        values.dedup();
    }
    let mut edges = vec![vec![]; positions.len()];
    for write in &graph.writes {
        if let Some(&pos) = positions.get(&write.binding) {
            // Each write owns one contiguous postorder range, including unsupported forms.
            for node in &graph.nodes[write.start..=write.value] {
                if let Node::Read(id) = node
                    && let Some(&dependency) = positions.get(id)
                {
                    edges[pos].push(dependency);
                }
            }
        }
    }
    let components = components(&edges);
    let mut membership = vec![0; positions.len()];
    for (component, members) in components.iter().enumerate() {
        for &member in members {
            membership[member] = component;
        }
    }
    let mut writes = vec![vec![]; components.len()];
    for (index, write) in graph.writes.iter().enumerate() {
        if let Some(&pos) = positions.get(&write.binding) {
            writes[membership[pos]].push((index, pos));
        }
    }
    let mut values = vec![Fact::Unset; graph.nodes.len()];
    let mut next = vec![Fact::Unset; positions.len()];
    let mut largest = 0;
    let mut at_limit = 0;
    for (component, members) in components.iter().enumerate() {
        let mut stable = false;
        for round in 1..=FACT_ROUNDS {
            largest = largest.max(round);
            if round == FACT_ROUNDS {
                at_limit += 1;
            }
            for &member in members {
                next[member] = Fact::Unset;
            }
            for &(index, pos) in &writes[component] {
                let write = &graph.writes[index];
                if graph.nodes[write.start..=write.value]
                    .iter()
                    .any(|node| matches!(node, Node::Read(id) if bindings[*id] == Fact::Unset))
                {
                    continue;
                }
                evaluate_range(graph, bindings, &mut values, write.start..=write.value);
                next[pos] = next[pos].join(values[write.value]);
            }
            let mut changed = false;
            for &member in members {
                let id = graph.bindings[member];
                let candidate = &mut next[member];
                if let (Fact::Integer(lo, hi), Fact::Integer(old_lo, old_hi)) =
                    (*candidate, bindings[id])
                {
                    // The fixed integer domain bounds this search to 54 comparisons.
                    let hi = if hi > old_hi {
                        thresholds[member][thresholds[member].partition_point(|t| *t < hi)]
                    } else {
                        old_hi
                    };
                    *candidate = Fact::Integer(if lo < old_lo { 0 } else { old_lo }, hi);
                }
                changed |= bindings[id] != *candidate;
            }
            for &member in members {
                bindings[graph.bindings[member]] = next[member];
            }
            if !changed {
                stable = true;
                break;
            }
        }
        // An unset member leaves some writes out of the join of this component.
        let incomplete = members
            .iter()
            .any(|&member| bindings[graph.bindings[member]] == Fact::Unset);
        if !stable || incomplete {
            for &member in members {
                bindings[graph.bindings[member]] = Fact::None;
            }
        }
    }
    (largest, at_limit)
}
