//! Each case costs one small compiler call. Runtime tests separately check evaluation order.
use crate::{CONDITION_NODES, Limits, compile};

fn code(test: &str) -> String {
    compile(
        format!("function yes():boolean{{return true;}}export function main(a:number,b:number,c:boolean):void{{const xs=[true];if({test}){{print('yes');}}}}").as_bytes(),
        Limits::default(),
    ).unwrap().luau
}

#[test]
fn inline_conditions_keep_mixed_grouping_and_final_calls() {
    for test in [
        "a<b&&c",
        "a<b||c",
        "(a<b||c)&&a>0",
        "c&&(a<b||a>0)",
        "c&&yes()",
    ] {
        let luau = code(test);
        let condition = luau
            .lines()
            .find(|s| s.contains("if (") && s.ends_with(" then"))
            .unwrap();
        assert!(
            condition.contains(" and ") || condition.contains(" or "),
            "{test}\n{luau}"
        );
    }
}

#[test]
fn statements_split_conditions_without_hoisting_effects() {
    for test in ["c&&xs[0]", "yes()&&c", "c||xs[0]&&yes()"] {
        let luau = code(test);
        assert!(luau.contains(": boolean = "), "{luau}");
        assert!(
            !luau
                .lines()
                .any(|s| s.contains("if (") && s.contains("yes() and")),
            "{luau}"
        );
        let branch = luau.rfind(" then\n").unwrap();
        assert!(branch > luau.find("local __ulua_g2_").unwrap(), "{luau}");
    }
}

#[test]
fn condition_node_bound_splits_the_next_boolean_operand() {
    for count in [CONDITION_NODES.div_ceil(2), CONDITION_NODES.div_ceil(2) + 1] {
        let luau = code(&vec!["c"; count].join("&&"));
        assert_eq!(
            luau.contains("local __ulua_g2_"),
            count * 2 - 1 > CONDITION_NODES,
            "{luau}"
        );
    }
}
