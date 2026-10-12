//! Direct string member witnesses: Node for ASCII, fixed byte values for UTF-8.
#[allow(dead_code)]
#[path = "support/script.rs"]
mod support;
#[path = "support/p24.rs"]
mod witness;
use witness::Suite;

#[test]
fn string_members_match_node_and_byte_goldens() {
    let mut s = Suite::new();
    ascii(&mut s);
    bytes(&mut s);
    traps(&mut s);
    random(&mut s);
    s.finish();
}
fn ascii(s: &mut Suite) {
    let positions = [
        "-Infinity",
        "-99",
        "-3.9",
        "-1",
        "-0.9",
        "-0",
        "0",
        "0.9",
        "1.9",
        "3",
        "99",
        "NaN",
        "Infinity",
    ];
    for text in ["''", "'ababa'"] {
        for p in positions {
            for method in ["slice", "substring", "substr"] {
                s.case(&format!("return {text}.{method}({p});"));
                for q in ["-Infinity", "-2.9", "NaN", "2.9", "99", "Infinity"] {
                    s.case(&format!("return {text}.{method}({p},{q});"));
                }
            }
            for method in [
                "indexOf",
                "lastIndexOf",
                "includes",
                "startsWith",
                "endsWith",
            ] {
                for needle in ["''", "'a'", "'aba'", "'z'", "'.*%[]'", "'abababa'"] {
                    s.case(&format!("return `${{{text}.{method}({needle},{p})}}`;"));
                }
            }
            s.case(&format!("return {text}.charAt({p});"));
            for method in ["charCodeAt", "codePointAt", "at"] {
                let valid = text != "''"
                    && (matches!(p, "-0.9" | "-0" | "0" | "0.9" | "1.9" | "3" | "NaN")
                        || (method == "at" && matches!(p, "-3.9" | "-1")));
                if valid {
                    s.case(&format!("return `${{{text}.{method}({p})}}`;"));
                } else {
                    s.trap(&format!("{text}.{method}({p});"), "trap:index-out-of-range");
                }
            }
            for method in ["padStart", "padEnd"] {
                // An empty pad must bypass even an infinite requested length.
                s.case(&format!("return {text}.{method}({p},'');"));
                if p != "Infinity" {
                    for pad in ["'xy'", "' '"] {
                        s.case(&format!("return {text}.{method}({p},{pad});"));
                    }
                    s.case(&format!("return {text}.{method}({p});"));
                }
            }
            if matches!(p, "-Infinity" | "-99" | "-3.9" | "-1" | "Infinity") {
                s.trap(&format!("{text}.repeat({p});"), "trap:invalid-argument");
            } else {
                s.case(&format!("return {text}.repeat({p});"));
            }
        }
        s.case(&format!("return {text}.slice();"));
        s.case(&format!("return {text}.concat('xyz');"));
        for method in [
            "indexOf",
            "lastIndexOf",
            "includes",
            "startsWith",
            "endsWith",
        ] {
            s.case(&format!("return `${{{text}.{method}('a')}}`;"));
        }
    }
    for text in ["''", "'ababa'", "',a,,b,'", "'a.*%[]b.*%[]'"] {
        for separator in ["''", "','", "'aba'", "'.*%[]'", "'missing'"] {
            s.case(&format!("return {text}.split({separator}).join('|');"));
            for limit in [
                "0",
                "-1",
                "-2.9",
                "NaN",
                "Infinity",
                "-Infinity",
                "0.9",
                "1.9",
                "2",
                "4294967296",
                "4294967297",
                "-4294967295",
            ] {
                s.case(&format!(
                    "return {text}.split({separator},{limit}).join('|');"
                ));
            }
        }
    }
    for method in ["trim", "trimStart", "trimEnd", "trimLeft", "trimRight"] {
        for text in ["''", "' x '", "'\\t\\r\\nx\\n\\r\\t'", "'abc'"] {
            s.case(&format!("return {text}.{method}();"));
        }
        for cp in [
            9, 10, 11, 12, 13, 32, 160, 5760, 8192, 8193, 8194, 8195, 8196, 8197, 8198, 8199, 8200,
            8201, 8202, 8232, 8233, 8239, 8287, 12288, 65279, 133,
        ] {
            s.case(&format!("return '\\u{{{cp:x}}}x\\u{{{cp:x}}}'.{method}();"));
        }
    }
    for method in ["replace", "replaceAll"] {
        for text in ["''", "'abaaba'", "'a.*%[]b.*%[]'"] {
            for pattern in ["''", "'a'", "'aba'", "'.*%[]'", "'missing'"] {
                for replacement in [
                    "''",
                    "'X'",
                    "'$$'",
                    "'$&'",
                    "'$`'",
                    "\"$'\"",
                    "'$1$9$0$10'",
                    "\"[$$][$&][$`][$'][$1]\"",
                    "'$$&$$$&$'",
                    "'é😀'",
                ] {
                    s.case(&format!("return {text}.{method}({pattern},{replacement});"));
                }
            }
        }
    }
    s.case("let s='a';s+=2;s+=false;s+='z';return s;");
    s.case("const xs=['a'];xs[0]+=2;xs[0]+=true;return xs[0];");
    s.case("return 'x'+-0+NaN+Infinity+-Infinity+1e-7+true+false+1e21;");
    s.case("return 1+'x'+true+'x';");
    s.case("let s='a';function right():string{s='b';return 'c';}return s+right()+s;");
    s.case("let s='a';function right():number{s='b';return 2;}s+=right();return s;");
    s.case("let s='abc';function from():number{s='xyz';return 1;}return s.slice(from())+s;");
    s.case("let order=0;function receiver():string{order=order*10+1;return 'abc';}function start():number{order=order*10+2;return 0;}function end():number{order=order*10+3;return 2;}const out=receiver().slice(start(),end());return `${order}:${out}`;");
    s.case("let text='abc';let out='';const fs:(()=>string)[]=[];for(const c of text){fs.push(()=>c);text='x';if(c==='b'){continue;}out+=c;}return out+fs.map((f:()=>string):string=>f()).join();");
}
fn bytes(s: &mut Suite) {
    for (body, expected) in [
        ("return `${'a😀é'.length}`;", "7"),
        ("return 'a😀é'.slice(1,5);", "😀"),
        ("return 'a😀é'.substring(5,1);", "😀"),
        ("return 'a😀é'.substr(-6,4);", "😀"),
        ("return `${'a😀é😀'.indexOf('😀',5)}`;", "7"),
        ("return `${'a😀é😀'.lastIndexOf('😀',5)}`;", "1"),
        ("return `${'a😀é'.includes('é',5)}`;", "true"),
        ("return `${'a😀é'.startsWith('😀',1)}`;", "true"),
        ("return `${'a😀é'.endsWith('😀',5)}`;", "true"),
        ("return 'a😀é'.charAt(1);", "😀"),
        ("return `${'a😀é'.charCodeAt(1)}`;", "240"),
        ("return `${'a😀é'.codePointAt(1)}`;", "128512"),
        ("return 'a😀é'.at(-6);", "😀"),
        ("return 'a😀é'.split('').join('|');", "a|😀|é"),
        ("return 'é😀é'.split('é').join('|');", "|😀|"),
        ("return 'é'.repeat(2);", "éé"),
        ("return 'é'.padStart(6,'😀');", "😀é"),
        ("return 'é'.padEnd(8,'😀é');", "é😀é"),
        ("return 'é'.concat('😀');", "é😀"),
        ("return 'é😀é'.replace('é','[$&]');", "[é]😀é"),
        ("return 'é😀é'.replaceAll('é','[$&]');", "[é]😀[é]"),
        ("return 'é😀'.replaceAll('','_');", "_é_😀_"),
        ("return 'é😀'.replace('','_');", "_é😀"),
        (
            "let out='';for(const c of 'a😀é'){out+='['+c+']';}return out;",
            "[a][😀][é]",
        ),
    ] {
        s.bytes(body, expected);
    }
    for method in ["trim", "trimStart", "trimEnd", "trimLeft", "trimRight"] {
        s.bytes(&format!("return 'é😀'.{method}();"), "é😀");
    }
    s.bytes("return 'é😀'.split('',1).join('|');", "é");
}
fn traps(s: &mut Suite) {
    for body in [
        "'é'.slice(1);",
        "'é'.slice(0,1);",
        "'é'.slice(2,1);",
        "'é'.substring(1);",
        "'é'.substr(0,1);",
        "'é'.indexOf('',1);",
        "'é'.lastIndexOf('',1);",
        "'é'.includes('',1);",
        "'é'.startsWith('',1);",
        "'é'.endsWith('',1);",
        "'é'.charAt(1);",
        "'é'.charCodeAt(1);",
        "'é'.codePointAt(1);",
        "'é'.at(-1);",
        "'x'.padStart(2,'é');",
        "'x'.padEnd(2,'é');",
    ] {
        s.trap(body, "trap:string-boundary");
    }
    for body in ["''.at(0);", "''.charCodeAt(0);", "''.codePointAt(0);"] {
        s.trap(body, "trap:index-out-of-range");
    }
    for body in [
        "'x'.repeat(-1);",
        "'x'.repeat(Infinity);",
        "'x'.repeat(-Infinity);",
    ] {
        s.trap(body, "trap:invalid-argument");
    }
}
fn random(s: &mut Suite) {
    let before = (s.count, s.comparable, s.traps, s.rejects);
    let mut state = 0x24a38_u64;
    for i in 0..384 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        if i >= 320 {
            let n = (state % 5) as usize;
            let body = format!(
                "const s='é😀'.repeat({n});return `${{s.length}}:${{s.split('').length}}:${{s.replaceAll('','_').length}}`; "
            );
            s.bytes(&body, &format!("{}:{}:{}", n * 6, n * 2, n * 8 + 1));
            continue;
        }
        let length = (state >> 32) % 9;
        let text: String = (0..length)
            .map(|j| char::from(b'a' + ((state >> (j * 5)) % 4) as u8))
            .collect();
        let p = ["-Infinity", "-2.9", "NaN", "0.9", "3.9", "Infinity"][(state % 6) as usize];
        let body = match i % 8 {
            0 => format!("return s.slice({p},3).substring(0,2).substr(-1);"),
            1 => format!(
                "return `${{s.indexOf('a',{p})}}:${{s.lastIndexOf('',{p})}}:${{s.includes('b',{p})}}`;"
            ),
            2 => format!(
                "return `${{s.startsWith('',{p})}}:${{s.endsWith('a',{p})}}:${{s.charAt({p})}}`;"
            ),
            3 => format!(
                "return s.split('a',{}).join('|').padStart(12,'xy').padEnd(15,'z');",
                state % 5
            ),
            4 => "return s.replaceAll('a',\"$$$&$`$'$1\").replace('','X').repeat(2);".into(),
            5 => {
                "let out='';for(const c of s){out+=c+2+true;}return out.trim().concat('x');".into()
            }
            6 => "return s.padEnd(12,'').split('').join('-');".into(),
            _ => "return s.toUpperCase();".into(),
        };
        s.source(&format!("function score():string{{const s='{text}';{body}}}export function main():void{{print(score());}}"), None);
    }
    eprintln!(
        "P2.4 random: programs=384 accepted={} node={} bytes={} traps={} rejected={}",
        s.count - before.0,
        s.comparable - before.1,
        64,
        s.traps - before.2,
        s.rejects - before.3
    );
}

#[test]
fn huge_string_allocations_use_limits_and_recover() {
    use ulua::*;
    for expression in [
        "'x'.repeat(1e300)",
        "'x'.padStart(1e300)",
        "'x'.padEnd(Infinity)",
    ] {
        for steps in [5000, 10_000_000] {
            let source = format!(
                "export function bad():number{{return {expression}.length;}}export function good():number{{return 7;}}export function main():void{{}}"
            );
            let mut script = support::Script::new(&source);
            let mut limits = RunLimits::default();
            limits.max_memory_bytes = if steps == 5000 { 64 << 20 } else { 1 << 20 };
            limits.max_steps = steps;
            assert_eq!(script.instance(limits).0, ULUA_OK);
            for _ in 0..2 {
                let (status, code, _) = script.call(script.index("bad"), &[]);
                assert_eq!(status, ULUA_FAILED);
                eprintln!("P2.4 allocation: expression={expression} steps={steps} code={code}");
                assert!(
                    matches!(code.as_str(), "limit:memory" | "limit:steps"),
                    "{expression}: {code}"
                );
                let (status, code, value) = script.call(script.index("good"), &[]);
                assert_eq!((status, code.as_str(), value.number), (ULUA_OK, "", 7.0));
            }
        }
    }
    for (expression, kind) in [
        ("'é'.slice(1)", "trap:string-boundary"),
        ("''.at(0)", "trap:index-out-of-range"),
        ("'x'.repeat(-1)", "trap:invalid-argument"),
    ] {
        let source = format!(
            "export function bad():number{{return {expression}.length;}}export function good():number{{return 7;}}export function main():void{{}}"
        );
        let mut script = support::Script::new(&source);
        assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
        for _ in 0..2 {
            let (status, code, _) = script.call(script.index("bad"), &[]);
            assert_eq!((status, code.as_str()), (ULUA_FAILED, kind));
            let (status, code, value) = script.call(script.index("good"), &[]);
            assert_eq!((status, code.as_str(), value.number), (ULUA_OK, "", 7.0));
        }
    }
}
