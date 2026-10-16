//! Spill probes use the production loop. No native compiler, VM, or external witness runs here.
use super::{functions::emit_with_plan_hook, output_bound};
use crate::{DEPTH_CEILING, Diagnostic, OUTPUT_FACTOR, checker, ir};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::time::Instant;

fn checked(
    source: &str,
    emit: impl FnOnce(&ir::Module) -> Result<String, Diagnostic>,
) -> Result<String, Diagnostic> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts().with_module(true)).parse();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = checker::check(&parsed.program, source.as_bytes(), DEPTH_CEILING)?;
    emit(&module)
}

// Cost: one small parse/check and four probes. The hook creates an actual two-state cycle.
#[test]
fn non_monotone_spill_plan_reaches_the_iteration_cap() {
    let start = Instant::now();
    let mut plans = Vec::new();
    let error = checked(
        "export function main(n:number):number{let x=1;return n+x;}",
        |module| {
            let owner = module.functions[0].binding + 1;
            let ids: Vec<_> = module
                .bindings
                .iter()
                .enumerate()
                .filter(|(_, binding)| binding.owner == owner)
                .map(|(id, _)| id)
                .collect();
            assert_eq!(ids.len(), 2);
            emit_with_plan_hook(
                module,
                output_bound(module, OUTPUT_FACTOR)?,
                false,
                &mut |frame, iteration, cap, spills| {
                    if frame == owner {
                        assert_eq!(cap, 3);
                        assert_eq!(iteration, plans.len() + 1);
                        spills.clear();
                        if iteration % 2 == 1 {
                            spills.insert(ids[1]);
                        }
                        plans.push(spills.clone());
                    }
                },
            )
        },
    )
    .unwrap_err();
    assert_eq!(plans.len(), 3);
    assert_eq!(plans[0], plans[2]);
    assert!(!plans[0].is_empty());
    assert!(plans[1].is_empty());
    assert_eq!(error.code, "internal");
    assert_eq!(
        error.message,
        "Spill plan did not converge within the iteration cap"
    );
    eprintln!(
        "non-monotone spill plan iterations=3 cap=3 cost={:.6}s",
        start.elapsed().as_secs_f64()
    );
}

// Cost: 53 parse/check/emission calls cover existing local, capture, block, operand, and loop spill shapes.
#[test]
fn existing_spill_shapes_stay_within_the_iteration_cap() {
    let start = Instant::now();
    let mut sources = Vec::new();
    for count in 150..=180 {
        let declarations = (0..count)
            .map(|i| format!("let v{i}=3;"))
            .collect::<String>();
        sources.push(format!(
            "export function main():void{{{declarations}for(let i=0;i<v{};i++){{}}}}",
            count - 1
        ));
    }
    for count in [0, 170, 180, 200, 400] {
        let declarations = (0..count)
            .map(|i| format!("let v{i}={i};"))
            .collect::<String>();
        sources.push(format!("{declarations}export function main():void{{}}"));
        sources.push(format!(
            "export function main():void{{{declarations}{{let nested=1;}}}}"
        ));
    }
    for count in [10, 11, 130, 190] {
        for blocks in [false, true] {
            if blocks && count > 11 {
                continue;
            }
            let declarations = (0..count)
                .map(|i| format!("{}const v{i}={i};", if blocks { "{" } else { "" }))
                .collect::<String>();
            let reads = (0..count).map(|i| format!("s+=v{i};")).collect::<String>();
            let close = if blocks {
                "}".repeat(count)
            } else {
                String::new()
            };
            sources.push(format!("export function main():number{{{declarations}function get():number{{let s=0;{reads}return s;}}return get();{close}}}"));
        }
    }
    for captured in [false, true] {
        for continues in [false, true] {
            let declarations = (0..200).map(|i| format!("let v{i}=3;")).collect::<String>();
            let capture = if captured { "const read=()=>i;" } else { "" };
            let jump = if continues {
                "if(i===1){continue;}"
            } else {
                ""
            };
            sources.push(format!("export function main():void{{{declarations}for(let i=0;i<v199;i++){{{capture}{jump}}}}}"));
        }
    }
    let declarations = (0..200)
        .map(|i| format!("let v{i}={i};"))
        .collect::<String>();
    sources.push(format!(
        "export function main():number{{{declarations}return v199+1;}}"
    ));
    let blocks = (0..220)
        .map(|i| format!("{{let x{i}={i};x{i}++;}}"))
        .collect::<String>();
    sources.push(format!("export function main():void{{{blocks}}}"));
    assert_eq!(sources.len(), 53);
    let mut largest = 0;
    let mut largest_cap = 0;
    let mut spill_plans = 0;
    for source in &sources {
        checked(source, |module| {
            emit_with_plan_hook(
                module,
                output_bound(module, OUTPUT_FACTOR)?,
                false,
                &mut |owner, iteration, cap, spills| {
                    // Anonymous functions have no storage candidate. These fixtures have no catch bindings.
                    let candidates = module
                        .bindings
                        .iter()
                        .enumerate()
                        .filter(|(id, b)| {
                            b.owner == owner
                                && !module
                                    .functions
                                    .iter()
                                    .any(|f| f.binding == *id && f.name.is_empty())
                        })
                        .count();
                    assert_eq!(cap, candidates + 1);
                    assert!((1..=cap).contains(&iteration), "{source}");
                    if iteration > largest {
                        largest = iteration;
                        largest_cap = cap;
                    }
                    spill_plans += usize::from(!spills.is_empty());
                },
            )
        })
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
    assert!(spill_plans > 0);
    assert!(largest > 1);
    eprintln!(
        "existing spill shapes cases={} largest_iterations={largest} corresponding_cap={largest_cap} cost={:.6}s",
        sources.len(),
        start.elapsed().as_secs_f64()
    );
}
