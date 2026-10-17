#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use std::time::Instant;
use ulua::{RunLimits, ULUA_FAILED, ULUA_OK, ULUA_TYPE_STRING, Value};
fn text(s: &str) -> Value {
    let mut v = Value::default();
    v.r#type = ULUA_TYPE_STRING;
    v.data = s.as_ptr().cast();
    v.length = s.len();
    v
}
fn returned(v: Value) -> String {
    // SAFETY: The caller reads the result before the next instance call.
    unsafe {
        String::from_utf8(std::slice::from_raw_parts(v.data.cast::<u8>(), v.length).to_vec())
            .unwrap()
    }
}
fn limits() -> RunLimits {
    let mut l = RunLimits::default();
    l.max_steps = 100_000_000;
    l.max_memory_bytes = 64 * 1024 * 1024;
    l
}
#[test]
fn json_stringify_and_parse_node_random_ten_types() {
    let mut s = support::Script::new(include_str!("../../test-data/json-oracle.ts"));
    assert_eq!(s.instance(limits()).0, ULUA_OK);
    for row in include_str!("../../test-data/json-oracle.tsv")
        .lines()
        .filter(|s| !s.starts_with('#'))
    {
        let fields: Vec<_> = row.split('\t').collect();
        let expected = String::from_utf8(
            fields[2]
                .as_bytes()
                .chunks_exact(2)
                .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
                .collect(),
        )
        .unwrap();
        let (status, code, value) = s.call(
            s.index(&format!("s{}", fields[0])),
            &[support::number(fields[1].parse().unwrap())],
        );
        assert_eq!((status, code), (ULUA_OK, String::new()), "{row}");
        assert_eq!(returned(value), expected, "stringify {row}");
        let (status, code, value) = s.call(s.index(&format!("p{}", fields[0])), &[text(&expected)]);
        assert_eq!((status, code), (ULUA_OK, String::new()), "{row}");
        assert_eq!(returned(value), expected, "parse {row}");
    }
}
const PARSE: &str = r#"type R={x:number};export function main(text:string):string{try{const v:R=JSON.parse(text);return JSON.stringify(v);}catch(e){if(e instanceof SyntaxError){return "syntax";}if(e instanceof TypeError){return "type";}return "other";}}"#;
#[test]
fn json_errors_hostile_prefixes_and_memory_limits() {
    let mut s = support::Script::new(PARSE);
    assert_eq!(s.instance(limits()).0, ULUA_OK);
    let valid = r#"{"extra":[null,true,"\ud83d\ude00",{"z":-1.25e+3}],"x":1}"#;
    for end in 0..valid.len() {
        let (status, code, v) = s.call(s.index("main"), &[text(&valid[..end])]);
        assert_eq!((status, code), (ULUA_OK, String::new()));
        assert_eq!(returned(v), "syntax", "prefix {end}");
    }
    for (input, expected) in [
        ("{\"x\":1e400}", "type"),
        ("{\"x\":1e400,}", "syntax"),
        ("{\"x\":null}", "type"),
        ("[]", "type"),
        ("{}", "type"),
        (r#"{"x":1,"z":"\ud800"}"#, "syntax"),
        (r#"{"x":1,"z":"\udc00"}"#, "syntax"),
        (r#"{"x":1,"x":2}"#, "{\"x\":2}"),
    ] {
        let (status, code, v) = s.call(s.index("main"), &[text(input)]);
        assert_eq!((status, code), (ULUA_OK, String::new()));
        assert_eq!(returned(v), expected, "{input}");
    }
    for input in [
        format!("{}0{}", "[".repeat(1024), "]".repeat(1024)),
        format!("{{\"x\":1,\"extra\":\"{}\"}}", "x".repeat(1_000_000)),
        format!(
            "{{{}\"x\":1}}",
            (0..20000)
                .map(|i| format!("\"k{i}\":{i},"))
                .collect::<String>()
        ),
    ] {
        let (status, code, _) = s.call(s.index("main"), &[text(&input)]);
        assert!(
            status == ULUA_OK
                || (status == ULUA_FAILED
                    && ["limit:memory", "limit:steps"].contains(&code.as_str())),
            "{status}/{code}"
        );
    }
    let mut limited = limits();
    limited.max_memory_bytes = 1024 * 1024;
    assert_eq!(s.instance(limited).0, ULUA_OK);
    let input = format!("{{\"x\":1,\"extra\":\"{}\"}}", "x".repeat(600_000));
    let (status, code, _) = s.call(s.index("main"), &[text(&input)]);
    assert_eq!((status, code), (ULUA_FAILED, "limit:memory".into()));
}
#[test]
fn json_container_depth_128_and_129() {
    let mut s = support::Script::new(
        r#"class Link{next:Link|null=null;}export function stringify(n:number):string{const root=new Link();let p:Link=root;for(let i=1;i<n;i++){const q=new Link();p.next=q;p=q;}return JSON.stringify(root);}type R={next:R|null};export function parse(text:string):string{try{const r:R=JSON.parse(text);return "ok";}catch(e){return e instanceof SyntaxError?"syntax":"other";}}"#,
    );
    assert_eq!(s.instance(limits()).0, ULUA_OK);
    for n in [128, 129] {
        let (status, code, value) = s.call(s.index("stringify"), &[support::number(n)]);
        if n == 128 {
            assert_eq!((status, code), (ULUA_OK, String::new()));
            assert_eq!(returned(value).matches("next").count(), 128);
        } else {
            assert_eq!((status, code), (ULUA_FAILED, "limit:json-depth".into()));
        }
        let input = format!("{}null{}", "{\"next\":".repeat(n), "}".repeat(n));
        let (status, code, value) = s.call(s.index("parse"), &[text(&input)]);
        assert_eq!((status, code), (ULUA_OK, String::new()));
        assert_eq!(returned(value), if n == 128 { "ok" } else { "syntax" });
    }
}
#[test]
fn json_release_throughput() {
    let mut s = support::Script::new(
        "type R={xs:number[]};export function stringify(n:number):number{const xs:number[]=[];for(let i=0;i<n;i++){xs.push(i);}const r:R={xs:xs};let bytes=0;for(let i=0;i<100;i++){bytes=bytes+Utf8.byteLength(JSON.stringify(r));}return bytes;}export function parse(text:string):number{let n=0;for(let i=0;i<100;i++){const r:R=JSON.parse(text);n=n+r.xs.length;}return n;}",
    );
    let input = format!(
        "{{\"xs\":[{}]}}",
        (0..10000)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    for (name, args) in [
        ("stringify", vec![support::number(10000)]),
        ("parse", vec![text(&input)]),
    ] {
        assert_eq!(s.instance(limits()).0, ULUA_OK);
        let start = Instant::now();
        let (status, code, value) = s.call(s.index(name), &args);
        let elapsed = start.elapsed().as_secs_f64();
        assert_eq!((status, code), (ULUA_OK, String::new()));
        assert_eq!(
            value.number,
            if name == "parse" {
                1_000_000.0
            } else {
                (input.len() * 100) as f64
            }
        );
        eprintln!(
            "JSON {name}: {:.3} MB/s; {:.3}s; {} bytes; release={}",
            input.len() as f64 * 100.0 / elapsed / 1e6,
            elapsed,
            input.len() * 100,
            !cfg!(debug_assertions)
        );
    }
}
#[test]
fn json_order_survives_related_nested_type_views_and_shared_children() {
    let source = r#"type A={z:number;a:number};type B={a:number;z:number};type Wrap={xs:B[];opt:B|null};function f(text:string):B{return JSON.parse(text);}export function main():string{const r:Wrap=JSON.parse('{"xs":[{"z":1,"a":2}],"opt":{"z":3,"a":4}}');const v:B=f('{"z":5,"a":6}');return `${JSON.stringify(r)}/${JSON.stringify(v)}`;}"#;
    let mut s = support::Script::new(source);
    assert_eq!(s.instance(limits()).0, ULUA_OK);
    let (status, code, v) = s.call(s.index("main"), &[]);
    assert_eq!((status, code), (ULUA_OK, String::new()));
    assert_eq!(
        returned(v),
        r#"{"xs":[{"a":2,"z":1}],"opt":{"a":4,"z":3}}/{"a":6,"z":5}"#
    );
    let mut s = support::Script::new(
        r#"class Link{next:Link|null=null;}type Pair={a:Link;b:Link};export function main():string{const x=new Link();const p:Pair={a:x,b:x};return JSON.stringify(p);}"#,
    );
    assert_eq!(s.instance(limits()).0, ULUA_OK);
    let (status, code, v) = s.call(s.index("main"), &[]);
    assert_eq!((status, code), (ULUA_OK, String::new()));
    assert_eq!(returned(v), r#"{"a":{"next":null},"b":{"next":null}}"#);
}
