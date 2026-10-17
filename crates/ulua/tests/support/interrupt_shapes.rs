//! Source-ceiling shapes expose forward paths, constructors, and nested function work.
use ulua_compiler::{Limits, SOURCE_CEILING, compile};
#[path = "interrupt_round2.rs"]
pub mod round2;

#[allow(dead_code)]
#[path = "p25_shapes.rs"]
mod p25;

#[allow(dead_code)]
#[path = "growth_shapes.rs"]
mod growth_shapes;

#[allow(dead_code)]
#[path = "table_shapes.rs"]
mod tables;

pub fn fill_at(bytes: usize, prefix: &str, unit: &str, suffix: &str) -> String {
    assert!(prefix.len() + suffix.len() <= bytes);
    let mut source = prefix.to_owned();
    while source.len() + unit.len() + suffix.len() <= bytes {
        source.push_str(unit);
    }
    source.push_str(suffix);
    source.extend(std::iter::repeat_n(' ', bytes - source.len()));
    source
}

pub fn arithmetic_terms() -> usize {
    for terms in 2..128 {
        let unit = format!("x={};", vec!["x"; terms].join("+"));
        let source = format!("export function main(b:boolean):void{{let x=1;{unit}}}");
        if let Err(error) = compile(source.as_bytes(), Limits::default()) {
            assert_eq!(error.code, "limit:nesting-depth");
            return terms - 1;
        }
    }
    panic!("arithmetic chain must reach the compiler depth limit");
}

fn arithmetic_at(bytes: usize, loop_call: bool) -> String {
    let fill = |prefix: &str, unit: &str, suffix: &str| fill_at(bytes, prefix, unit, suffix);
    let terms = arithmetic_terms();
    eprintln!("interrupt arithmetic terms={terms}");
    let unit = format!("x={};", vec!["x"; terms].join("+"));
    if loop_call {
        fill(
            "function f():void{let x=1;",
            &unit,
            "}export function main():void{while(true){f();}}",
        )
    } else {
        fill("export function main(b:boolean):void{let x=1;", &unit, "}")
    }
}

pub fn all() -> Vec<(&'static str, String)> {
    all_at(SOURCE_CEILING as usize)
}

pub fn all_at(bytes: usize) -> Vec<(&'static str, String)> {
    let fill = |prefix: &str, unit: &str, suffix: &str| fill_at(bytes, prefix, unit, suffix);
    let ceiling = bytes >= 32 * 1024;
    let number = "export function main(b:boolean):void{let x=1;";
    let mut shapes = vec![("arithmetic", arithmetic_at(bytes, false))];
    for (name, prefix, unit, suffix) in [
        (
            "concat",
            "export function main(b:boolean):void{let s='';",
            "s=s+'';",
            "}",
        ),
        ("increment", number, "x++;", "}"),
        (
            "math-native-loop",
            "export function main(b:boolean):void{let x=1;for(let i=0;i<2;i++){",
            "x=Math.sin(x);",
            "}}",
        ),
        (
            "math-prelude-loop",
            "export function main(b:boolean):void{let x=1;for(let i=0;i<2;i++){",
            "x=Math.round(x);",
            "}}",
        ),
        (
            "math-random-loop",
            "export function main(b:boolean):void{let x=1;for(let i=0;i<2;i++){",
            "x=Math.random();",
            "}}",
        ),
        (
            "array-find",
            "export function main(b:boolean):void{const xs=[1,2,3,4];",
            "xs.find((v:number,i:number):boolean=>i>9);",
            "}",
        ),
        (
            "array-findLast",
            "export function main(b:boolean):void{const xs=[1,2,3,4];",
            "xs.findLast((v:number,i:number):boolean=>i>9);",
            "}",
        ),
        ("remainder", number, "x=x%3;", "}"),
        ("remainder-compound", number, "x%=3;", "}"),
        (
            "boolean-template",
            "export function main(b:boolean):void{let s='';",
            "s=`${b}`;",
            "}",
        ),
        (
            "field-increment",
            "export function main(b:boolean):void{let o={a:0};",
            "o.a++;",
            "}",
        ),
        ("if", number, "if(b)x++;", "}"),
        (
            "nested-if",
            number,
            "if(b){if(b)x++;else x+=2;}else{if(b)x+=3;else x+=4;}",
            "}",
        ),
        (
            "try",
            number,
            "try{x++;x++;x++;x++;x++;x++;x++;x++;}catch(e){x++;}",
            "}",
        ),
        (
            "try-body",
            &format!("{number}try{{"),
            "x++;",
            "throw new Error('test');}catch(e){x++;}}",
        ),
        (
            "catch-body",
            &format!("{number}try{{throw new Error('test');}}catch(e){{"),
            "x++;",
            "}}",
        ),
        (
            "switch-if",
            number,
            "if(x<0)x++;else if(x<1)x++;else if(x<2)x++;else if(x<3)x++;else x++;",
            "}",
        ),
        (
            "loop-function",
            "export function main(b:boolean):void{for(let i=0;i<1;i++){function f():void{let x=1;",
            "x++;",
            "}f();}}",
        ),
    ] {
        shapes.push((name, fill(prefix, unit, suffix)));
    }
    let array = fill(
        "export function main(b:boolean):void{let x=1;let a=[",
        "x+x,",
        "x+x];}",
    );
    shapes.push(("computed-array", array));
    let fields = (0..if ceiling { 1024 } else { 64 })
        .map(|i| format!("a{i}:x+x"))
        .collect::<Vec<_>>()
        .join(",");
    shapes.push((
        "computed-object",
        fill(number, &format!("{{let o={{{fields}}};}}"), "}"),
    ));
    for (name, op, atom, init) in [
        ("and", "&&", "b", "let x=false;"),
        ("or", "||", "b", "let x=false;"),
        (
            "coalesce",
            "??",
            "a",
            "let a:R|null=b?{n:1}:null;let x:R|null=null;",
        ),
    ] {
        let expression = vec![atom; 32].join(op);
        shapes.push((
            name,
            fill(
                &format!("type R={{n:number}};export function main(b:boolean):void{{{init}"),
                &format!("x={expression};"),
                "}",
            ),
        ));
    }
    shapes.push(("loop-exit", fill(number, "while(b){x++;break;}x++;", "}")));
    shapes.push((
        "branch-join",
        fill(
            "function f():void{}export function main(b:boolean):void{let x=1;",
            "if(b){x++;x++;x++;}else{f();}x++;",
            "}",
        ),
    ));
    shapes.push((
        "finally-body",
        fill(&format!("{number}try{{x++;}}finally{{"), "x++;", "}}"),
    ));
    let mut balanced = "b".to_owned();
    for _ in 0..8 {
        balanced = format!("({balanced}&&{balanced})");
    }
    shapes.push((
        "balanced-and",
        fill(
            "export function main(b:boolean):void{let x=false;",
            &format!("x={balanced};"),
            "}",
        ),
    ));
    let mut declarations = number.to_owned();
    let mut index = 0;
    loop {
        let unit = format!("let a{index}=0;");
        if declarations.len() + unit.len() + 1 > bytes {
            break;
        }
        declarations.push_str(&unit);
        index += 1;
    }
    shapes.push(("spill-headers", fill(&declarations, " ", "}")));
    let mut exports = String::new();
    for i in 0..if ceiling { 1200 } else { 32 } {
        exports.push_str(&format!("export function f{i}():void{{}}"));
    }
    shapes.push((
        "exports",
        fill(&exports, " ", "export function main(b:boolean):void{}"),
    ));
    let mut try_boundary = String::new();
    for count in 0..16 {
        try_boundary.push_str(&"o.a++;".repeat(count));
        try_boundary.push_str("try{o.a++;}catch(e){o.a++;}");
    }
    shapes.push((
        "try-boundary",
        fill(
            "export function main(b:boolean):void{let o={a:0};",
            &try_boundary,
            "}",
        ),
    ));
    let parameters = (0..32)
        .map(|i| format!("p{i}:number=0"))
        .collect::<Vec<_>>()
        .join(",");
    // Nested argument lists crowd the local budget without a source-ceiling body.
    let inner = format!(
        "{},{}",
        vec!["one()"; 31].join(","),
        vec!["x"; 36].join("+")
    );
    let arguments = if ceiling {
        vec![vec!["x"; 40].join("+"); 32].join(",")
    } else {
        format!("{},g({inner})", vec!["one()"; 31].join(","))
    };
    let prefix = format!(
        "function one():number{{return 1;}}function g({parameters}):number{{return p0;}}function f({parameters}):void{{let x=1;"
    );
    let suffix = format!("g({arguments});}}export function main(b:boolean):void{{f();}}");
    let mut source = prefix;
    let mut index = 0;
    loop {
        let declaration = format!("let a{index}=0;");
        if source.len() + declaration.len() + suffix.len() > bytes {
            break;
        }
        source.push_str(&declaration);
        index += 1;
    }
    source.push_str(&suffix);
    source.extend(std::iter::repeat_n(' ', bytes - source.len()));
    shapes.push(("paged-spill", source));
    shapes.extend(tables::interrupts(bytes));
    shapes
}

/// Execute the growth forms that exposed loop joins in the analyzer.
pub fn analyzer_growth(bytes: usize) -> Vec<(String, String)> {
    growth_shapes::all()
        .into_iter()
        .filter(|shape| {
            [
                "S4-function-bindings",
                "S4-module-bindings",
                "S4-nested-bindings",
                "S4-nested-writes",
                "dense-template",
                "P25-nested",
                "dense-less",
                "dense-greater",
                "dense-less-equal",
                "dense-greater-equal",
                "dense-loose-equal",
                "dense-loose-unequal",
            ]
            .contains(&shape.label.as_str())
        })
        .map(|shape| {
            let mut source = (shape.make)(shape.key, bytes);
            if shape.label.starts_with("dense-") {
                // The growth witness only defines h. The instruction probe also executes its body.
                source = source.trim_end().strip_suffix("};}").unwrap().to_owned() + "};h();}";
                assert!(source.len() <= bytes);
                source.extend(std::iter::repeat_n(' ', bytes - source.len()));
            }
            (shape.label, source)
        })
        .collect()
}

/// Repeat all module initialization and function work of each affected growth form.
pub fn growth_loops(bytes: usize) -> Vec<(String, String)> {
    analyzer_growth(bytes).into_iter().map(|(name, source)| {
        let mut body = source.trim_end();
        let mut types = String::new();
        while body.starts_with("type ") {
            let mut braces = 0;
            let end = body.char_indices().find_map(|(index, ch)| {
                if ch == '{' { braces += 1; }
                if ch == '}' { braces -= 1; }
                (ch == ';' && braces == 0).then_some(index + 1)
            }).unwrap();
            types.push_str(&body[..end]);
            body = &body[end..];
        }
        let work = if name == "S4-module-bindings" {
            let body = body.replace("export function main():void{}", "");
            format!("{types}function work():void{{{body}}}")
        } else {
            let body = body.replace("export function main():void{", "function work():void{");
            assert!(body.contains("function work():void{"));
            types + &body
        };
        let mut source = work + "function start():number{while(true){work();}return 0;}const stop=start();export function main():void{}";
        assert!(source.len() <= SOURCE_CEILING as usize);
        source.extend(std::iter::repeat_n(' ', SOURCE_CEILING as usize - source.len()));
        (name, source)
    }).collect()
}

/// Wrapped conditions exercise both branch outcomes and the inline conditional instruction budget.
pub fn conditions(bytes: usize) -> Vec<(&'static str, String)> {
    let mut sources: Vec<_> = [
        "logical-wrapped-tests",
        "conditional-wrapped-tests",
        "alternating-logical-tests",
        "conditional-tests",
        "nullable-conditional-tests",
    ]
    .into_iter()
    .map(|name| {
        let mut source = p25::make(name, bytes)
            .trim_end()
            .to_owned()
            .replace(
                "export function main():void{",
                "export function main(b:boolean):void{",
            )
            .replace("const b:boolean=true;", "")
            .replace("const x:B|null={v:0};", "const x:B|null=b?{v:0}:null;");
        assert!(source.len() <= bytes);
        source.extend(std::iter::repeat_n(' ', bytes - source.len()));
        (name, source)
    })
    .collect();
    for (name, rhs) in [
        ("logical-wrapped-not", "!b"),
        ("logical-wrapped-right-tree", "(b && !b)"),
    ] {
        let mut condition = "b".to_owned();
        for level in 0..7 {
            condition = format!(
                "(!(({condition}{}{rhs})))",
                if level % 2 == 0 { "&&" } else { "||" }
            );
        }
        sources.push((
            name,
            fill_at(
                bytes,
                "export function main(b:boolean):void{",
                &format!("if({condition}){{}}"),
                "}",
            ),
        ));
    }
    sources
}

/// Repeat source work in the script loop, including both outcomes of boolean branches.
pub fn loop_source(source: &str) -> String {
    let work = source.trim_end().replace(
        "export function main(b:boolean):void{",
        "function work(b:boolean):void{",
    );
    assert!(work.contains("function work(b:boolean)"));
    let suffix = "function start():number{while(true){work(true);work(false);}return 0;}const stop=start();export function main():void{}";
    let mut out = work + suffix;
    assert!(out.len() <= SOURCE_CEILING as usize);
    out.extend(std::iter::repeat_n(
        ' ',
        SOURCE_CEILING as usize - out.len(),
    ));
    out
}

/// Small instances cross several interrupt boundaries and each constructor split boundary.
pub fn small() -> Vec<(String, String)> {
    let mut shapes: Vec<_> = all_at(4096)
        .into_iter()
        .filter(|(name, _)| *name != "paged-spill")
        .map(|(name, source)| (name.to_owned(), source))
        .collect();
    shapes.extend(
        conditions(4096)
            .into_iter()
            .map(|(name, source)| (name.to_owned(), source)),
    );
    shapes.extend(growth_loops(4096));
    shapes.extend(
        round2::all(16384)
            .into_iter()
            .filter(|(name, _)| !name.contains("-boundary-")),
    );
    let (_, paged) = all_at(20 * 1024)
        .into_iter()
        .find(|(name, _)| *name == "paged-spill")
        .unwrap();
    shapes.push(("paged-spill".to_owned(), paged));
    for count in 0..=3 {
        let exports = (0..count)
            .map(|i| format!("export function f{i}():void{{}}"))
            .collect::<Vec<_>>()
            .join("");
        shapes.push((format!("export-boundary-{count}"), exports.clone()));
        shapes.push((format!("export-loop-boundary-{count}"), exports));
        let fields = (0..count)
            .map(|i| format!("a{i}:x+x"))
            .collect::<Vec<_>>()
            .join(",");
        shapes.push((
            format!("object-boundary-{count}"),
            format!("export function main(b:boolean):void{{let x=1;let o={{{fields}}};}}"),
        ));
    }
    let body = "q++;".repeat(24);
    let protected = format!("let q=0;{body}");
    let source = format!(
        "export function main(b:boolean):void{{let q=0;function inner():void{{let q=0;for(let i=0;i<2;i++){{if(b){{{body}}}else{{{body}}}try{{{protected}}}catch(e){{{protected}}}finally{{{protected}}}}}}}if(b){{inner();}}else{{{body}}}while(b){{{body}break;}}{body}}}"
    );
    shapes.push(("nested-boundaries".to_owned(), source));
    for nodes in 1..=3 {
        let pure = vec!["b"; nodes].join("&&");
        let ternary = "b?x:".repeat(nodes) + "x";
        shapes.push((
            format!("pure-boundary-{nodes}"),
            format!(
                "export function main(b:boolean):void{{let x=1;let y=false;{} }}",
                format!("y={pure};x={ternary};").repeat(24)
            ),
        ));
    }
    shapes
}
