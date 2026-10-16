//! P1-P4 families, example scripts, string completions, and capture and local-budget spills.
pub struct Program {
    pub name: &'static str,
    pub source: String,
    pub grants: Vec<&'static str>,
    pub rejected: bool,
}
pub fn programs() -> Vec<Program> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let paths = [
        "corpus/accept/a07-loops.ts",                      // P1.
        "corpus/accept/a30-function-parameter-default.ts", // P2.1 functions.
        "corpus/accept/a09-closures.ts",                   // P2.2 closures.
        "corpus/accept/a13-arrays.ts",                     // P2.2 arrays.
        "corpus/accept/a33-array-callbacks.ts",            // P2.3 methods.
        "corpus/accept/a37-string-methods.ts",             // P2.4 strings.
        "corpus/accept/a14-nullable.ts",                   // P2.5 nullable values.
        "corpus/accept/a15-records.ts",                    // P2.5 records.
        "corpus/accept/a16-classes.ts",                    // P2.6 classes.
        "corpus/accept/a51-try-catch.ts",                  // P2.7 exceptions.
        "corpus/accept/a52-finally.ts",                    // P2.7 finally.
        "corpus/accept/a55-host-functions.ts",             // P3 grants.
        "corpus/accept/a58-async-order.ts",                // P4.1 async exports.
        "corpus/accept/a63-promise-all.ts",                // P4.2 promises.
        "corpus/accept/a64-host-requests.ts",              // P4.3 requests.
        "examples/hello/hello.ts",
        "examples/capabilities/game.ts",
        "examples/async/scores.ts",
        "examples/limits/limits.ts",
        "examples/run/hello.ts",
        "examples/run/async.ts",
        "examples/run/no-main.ts",
        "examples/run/trap.ts",
        "examples/run/init-trap.ts",
        "examples/run/reject.ts",
    ];
    let mut result: Vec<_> = paths
        .into_iter()
        .map(|name| {
            let source = std::fs::read_to_string(root.join(name)).unwrap();
            let grants = match name {
                "examples/capabilities/game.ts" => vec!["damage", "log"],
                "examples/async/scores.ts" => vec!["fetchScore"],
                _ => source
                    .lines()
                    .find_map(|s| s.strip_prefix("// grants:"))
                    .map(|s| {
                        s.split(',')
                            .map(|name| match name.trim() {
                                "hostAdd" => "hostAdd",
                                "hostConcat" => "hostConcat",
                                "hostNot" => "hostNot",
                                "hostLog" => "hostLog",
                                "hostLater" => "hostLater",
                                _ => panic!("test grant"),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            };
            Program {
                name,
                source,
                grants,
                rejected: name.ends_with("reject.ts"),
            }
        })
        .collect();
    result.push(Program {
        name: "string-completion",
        source: "export async function main(): Promise<string> { try { return await hostText(); } catch (e) { print(\"catch\"); return \"caught\"; } finally { print(\"finally\"); } }".into(),
        grants: vec!["hostText"], rejected: false,
    });
    result.push(Program {
        name: "protected-allocation",
        // The runtime string exceeds Luau's largest size class and reaches the native allocator.
        source: "export function main(): number { try { print(\"__fault_try\"); const text: string = \"x\".repeat(8192); return text.length; } catch (e) { print(\"catch\"); return 0; } finally { print(\"finally\"); } }".into(),
        grants: vec![], rejected: false,
    });
    result.push(spill_program("capture-spill", 11, true));
    result.push(spill_program("local-budget-spill", 177, false));
    result
}

fn spill_program(name: &'static str, count: usize, captured: bool) -> Program {
    // These source counts exceed CAPTURE_LOCALS (10) and LOCAL_BUDGET (176) independently.
    let mut source = String::from("export function main(): number {");
    for i in 0..count {
        source.push_str(&format!("const v{i} = {i};"));
    }
    if captured {
        source.push_str("function get(): number {");
    }
    source.push_str("let sum = 0;");
    for i in 0..count {
        source.push_str(&format!("sum += v{i};"));
    }
    source.push_str("return sum;");
    if captured {
        source.push_str("} return get();");
    }
    source.push('}');
    let compiled =
        ulua_compiler::compile(source.as_bytes(), ulua_compiler::Limits::default()).unwrap();
    assert!(compiled.luau.contains("__ulua_spill"), "{name}");
    Program {
        name,
        source,
        grants: vec![],
        rejected: false,
    }
}
