#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use ulua::{RunLimits, ULUA_FAILED, ULUA_OK};
fn run(body: &str) -> String {
    let source = format!("type R={{x:number}};export function main():string{{{body}}}");
    let mut s = support::Script::new(&source);
    let mut limits = RunLimits::default();
    limits.max_steps = 100_000_000;
    assert_eq!(s.instance(limits).0, ULUA_OK);
    let (status, code, v) = s.call(s.index("main"), &[]);
    assert_eq!((status, code), (ULUA_OK, String::new()), "{source}");
    // SAFETY: The returned string remains owned by the live instance until its next call.
    unsafe {
        String::from_utf8(std::slice::from_raw_parts(v.data.cast::<u8>(), v.length).to_vec())
            .unwrap()
    }
}
#[test]
fn ordered_key_equality_and_nullable_values() {
    assert_eq!(
        run(
            "const m=new Map<number,number|null>();m.set(1,null);m.set(2,9);let count=0;let total=0;for(const v of m.values()){count++;total=total+(v??0);}return `${count}/${total}`;"
        ),
        "2/9"
    );
    assert_eq!(
        run(
            "const s=new Set<number>([-0]);let sign=0;s.forEach((k:number):void=>{sign=1/k;});return `${sign}`;"
        ),
        "Infinity"
    );
    for (body, expected) in [
        (
            "const m=new Map<number,number>();m.set(NaN,1);m.set(NaN,2);m.set(-0,3);m.set(0,4);let zero=0;for(const k of m.keys()){if(k===0){zero=1/k;}}return `${m.size}/${m.getOr(NaN,0)}/${m.getOr(-0,0)}/${zero}`;",
            "2/2/4/Infinity",
        ),
        (
            "const m=new Map<string,number>();m.set('ab',1);m.set('a'+'b',2);return `${m.size}/${m.getOr('ab',0)}`;",
            "1/2",
        ),
        (
            "const s=new Set<boolean>([false,true,false]);let text='';for(const k of s){text=`${text}/${k}`;}return `${s.size}${text}`;",
            "2/false/true",
        ),
        (
            "const a:R={x:1};const b:R={x:1};const s=new Set<R>([a,b,a]);return `${s.size}/${s.has(a)}/${s.has({x:1})}`;",
            "2/true/false",
        ),
        (
            "const a=[1];const b=[1];const s=new Set<number[]>([a,b,a]);return `${s.size}/${s.has(a)}/${s.has([1])}`;",
            "2/true/false",
        ),
        (
            "const m=new Map<number,number|null>();m.set(1,null);return `${m.has(1)}/${m.getOr(1,8)===null}/${m.getOr(2,8)??0}`;",
            "true/true/8",
        ),
    ] {
        assert_eq!(run(body), expected);
    }
}
#[test]
fn ordered_overwrite_delete_reinsert_compaction_and_copy() {
    assert_eq!(
        run(
            "const m=new Map<number,number>();for(let i=0;i<16;i++){m.set(i,i);}m.set(0,99);for(let i=1;i<15;i++){m.delete(i);}m.set(1,100);const copy=new Map<number,number>(m);m.clear();let out='';copy.forEach((v:number,k:number):void=>{out=`${out}${k}:${v},`;});return `${m.size}/${copy.size}/${out}`;"
        ),
        "0/3/0:99,15:15,1:100,"
    );
}
#[test]
fn traversed_collection_mutation_traps_after_the_change() {
    for collection in ["Map", "Set"] {
        for traversal in ["each", "loop"] {
            for mutation in ["add", "delete", "clear"] {
                let (subject, change) = if collection == "Map" {
                    (
                        "m.keys()",
                        match mutation {
                            "add" => "m.set(2,2);",
                            "delete" => "m.delete(1);",
                            _ => "m.clear();",
                        },
                    )
                } else {
                    (
                        "m",
                        match mutation {
                            "add" => "m.add(2);",
                            "delete" => "m.delete(1);",
                            _ => "m.clear();",
                        },
                    )
                };
                let body = if traversal == "each" {
                    format!("m.forEach((v:number):void=>{{{change}}});")
                } else {
                    format!("for(const v of {subject}){{{change}}}")
                };
                let (declaration, setup) = if collection == "Map" {
                    ("const m=new Map<number,number>();", "m.set(1,1);")
                } else {
                    ("const m=new Set<number>([1]);", "")
                };
                let source = format!(
                    "{declaration}export function main():void{{{setup}{body}}}export function changed():number{{return m.size;}}export function recover():number{{m.clear();return m.size;}}"
                );
                let mut s = support::Script::new(&source);
                assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
                let (status, code, _) = s.call(s.index("main"), &[]);
                assert_eq!(
                    (status, code),
                    (ULUA_FAILED, "trap:map-modified".into()),
                    "{source}"
                );
                let (status, _, v) = s.call(s.index("changed"), &[]);
                assert_eq!(status, ULUA_OK);
                assert_eq!(v.number, if mutation == "add" { 2.0 } else { 0.0 });
                assert_eq!(s.call(s.index("recover"), &[]).0, ULUA_OK);
            }
        }
    }
}
#[test]
fn traversal_cleanup_on_break_return_throw_and_nested_loops() {
    for traversal in [
        "for(const k of m.keys()){break;}",
        "try{for(const k of m.keys()){throw new Error('x');}}catch(e){}",
        "for(const k of m.keys()){for(const j of m.keys()){break;}}",
    ] {
        assert_eq!(
            run(&format!(
                "const m=new Map<number,number>();m.set(1,1);{traversal}m.set(2,2);return `${{m.size}}`;"
            )),
            "2"
        );
    }
    let mut s = support::Script::new(
        "const m=new Set<number>([1]);export function main():number{for(const k of m){return k;}return 0;}export function after():number{m.add(2);return m.size;}",
    );
    assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
    assert_eq!(s.call(s.index("main"), &[]).0, ULUA_OK);
    assert_eq!(s.call(s.index("after"), &[]).2.number, 2.0);
    assert_eq!(
        run(
            "const m=new Map<number,number>();m.set(1,1);try{m.forEach((v:number):void=>{throw new Error('x');});}catch(e){}m.set(2,2);return `${m.size}`;"
        ),
        "2"
    );
}
#[test]
fn node_differential_ten_thousand_sequences() {
    let source = include_str!("../../test-data/map-oracle.ts");
    let mut s = support::Script::new(source);
    let mut limits = RunLimits::default();
    limits.max_steps = 100_000_000;
    assert_eq!(s.instance(limits).0, ULUA_OK);
    let index = s.index("main");
    let mut count = 0;
    for row in include_str!("../../test-data/map-oracle.tsv").lines() {
        let (seed, expected) = row.split_once('\t').unwrap();
        let (status, code, v) = s.call(index, &[support::number(seed.parse().unwrap())]);
        assert_eq!((status, code), (ULUA_OK, String::new()), "seed {seed}");
        // SAFETY: The live instance owns the returned string until the next call.
        let actual = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(v.data.cast::<u8>(), v.length)).unwrap()
        };
        assert_eq!(actual, expected, "seed {seed}");
        count += 1;
    }
    assert_eq!(count, 10_000);
    eprintln!(
        "P7.4 Node differential: {count} sequences, {} operations, zero differences",
        count * 32
    );
}
#[test]
fn ordered_collection_allocations_and_loops_are_limited() {
    let mut s = support::Script::new(
        "export function main():number{const m=new Map<number,number>();for(let i=0;i<100000;i++){m.set(i,i);}return m.size;}",
    );
    let mut limits = RunLimits::default();
    limits.max_steps = 1_000_000_000;
    limits.max_memory_bytes = 1024 * 1024;
    assert_eq!(s.instance(limits).0, ULUA_OK);
    assert_eq!(s.call(s.index("main"), &[]).1, "limit:memory");
    for body in [
        "const m=new Map<number,number>();for(let i=0;i<20000;i++){m.set(i,i);}const c=new Map<number,number>(m);",
        "const a:number[]=[];for(let i=0;i<20000;i++){a.push(i);}const s=new Set<number>(a);",
    ] {
        let mut s = support::Script::new(&format!("export function main():void{{{body}}}"));
        let mut limits = RunLimits::default();
        limits.max_steps = 10000;
        assert_eq!(s.instance(limits).0, ULUA_OK);
        assert_eq!(s.call(s.index("main"), &[]).1, "limit:steps");
    }
}
#[test]
#[ignore = "explicit Release product VM operation cost measurement"]
fn ordered_collection_operation_cost() {
    assert!(
        std::hint::black_box(!cfg!(debug_assertions)),
        "run with --release"
    );
    let _lock = crate::measurement_lock();
    const N: usize = 100_000;
    for key in ["number", "string"] {
        let key_at = if key == "number" { "i" } else { "keys[i]" };
        let setup = if key == "string" {
            format!("const keys:string[]=[];for(let i=0;i<{N};i++){{keys.push(`key${{i}}`);}}")
        } else {
            String::new()
        };
        for op in ["set", "get", "has", "delete", "iteration"] {
            let fill = format!("for(let i=0;i<{N};i++){{m.set({key_at},i);}}");
            let work = match op {
                "set" => format!("m.clear();{fill}"),
                "get" => format!("for(let i=0;i<{N};i++){{sum=sum+(m.get({key_at})??0);}}"),
                "has" => format!("for(let i=0;i<{N};i++){{if(m.has({key_at})){{sum++;}}}}"),
                "delete" => format!("for(let i=0;i<{N};i++){{m.delete({key_at});}}"),
                _ => "for(const v of m.values()){sum=sum+v;}".into(),
            };
            let source = format!(
                "{setup}const m=new Map<{key},number>();export function reset():void{{m.clear();{fill}}}export function main():number{{let sum=0;{work}return sum;}}"
            );
            // Module initialization only declares values; string-key setup lives in a function.
            let source = if key == "string" {
                source.replacen(&setup,&format!("const keys:string[]=[];function initialize():number{{{}}}const initialized=initialize();",setup.replace("const keys:string[]=[];","")+"return 0;"),1)
            } else {
                source
            };
            let mut s = support::Script::new(&source);
            let mut limits = RunLimits::default();
            limits.max_steps = 1_000_000_000;
            limits.max_memory_bytes = 128 * 1024 * 1024;
            assert_eq!(s.instance(limits).0, ULUA_OK);
            let mut samples = vec![];
            for _ in 0..5 {
                assert_eq!(s.call(s.index("reset"), &[]).0, ULUA_OK);
                let now = std::time::Instant::now();
                let (status, _, v) = s.call(s.index("main"), &[]);
                assert_eq!(status, ULUA_OK);
                std::hint::black_box(v.number);
                samples.push(now.elapsed().as_secs_f64() * 1e9 / N as f64);
            }
            samples.sort_by(f64::total_cmp);
            eprintln!(
                "P7.4 {key} {op}: {:.3} ns/operation, median of 5 x {N}; loop included",
                samples[2]
            );
        }
    }
}

#[test]
fn collection_absent_values_keep_optional_access_and_skip_arguments() {
    assert_eq!(
        run(
            "const outer=new Map<number,Map<number,number>>();const inner=new Map<number,number>();inner.set(1,1);outer.set(1,inner);const hit=outer.get(1);const miss=outer.get(2);let count=0;const side=():number=>{count++;return 9;};hit?.set(2,2);miss?.set(3,side());const size=hit?.size??0;const fallback=miss?.size??7;hit?.clear();return `${size}/${fallback}/${count}/${inner.size}`;"
        ),
        "2/7/0/0"
    );
    assert_eq!(
        run(
            "const outer=new Map<number,Set<number>>();outer.set(1,new Set<number>([1]));const hit=outer.get(1);hit?.add(2);return `${hit?.size??0}`;"
        ),
        "2"
    );
}

#[test]
fn present_keys_and_members_can_be_updated_during_traversal() {
    for traversal in [
        "m.forEach((v:number,k:number):void=>{m.set(k,v+1);});",
        "for(const k of m.keys()){m.set(k,m.getOr(k,0)+1);}",
        "for(const v of m.values()){m.set(v/10,v+1);}",
    ] {
        assert_eq!(
            run(&format!(
                "const m=new Map<number,number>();m.set(1,10).set(2,20).set(3,30);{traversal}let out='';m.forEach((v:number,k:number):void=>{{out=`${{out}}${{k}}:${{v}},`;}});return `${{out}}/${{m.size}}`;"
            )),
            "1:11,2:21,3:31,/3",
            "{traversal}"
        );
    }
    for traversal in [
        "for(const k of s){s.add(k);}",
        "s.forEach((k:number):void=>{s.add(k);});",
    ] {
        assert_eq!(
            run(&format!(
                "const s=new Set<number>([1,2,3]);{traversal}let out='';for(const k of s){{out=`${{out}}${{k}},`;}}return `${{out}}/${{s.size}}`;"
            )),
            "1,2,3,/3",
            "{traversal}"
        );
    }
}

#[test]
fn traversal_then_failed_export_preserves_mutation_traps() {
    for (declaration, setup, subject) in [
        ("new Map<number,number>()", "m.set(1,1);", "m.keys()"),
        ("new Map<number,number>()", "m.set(1,1);", "m.values()"),
        ("new Set<number>([1])", "", "m"),
    ] {
        let source = format!(
            "const m={declaration};async function value(n:number):Promise<number>{{return n;}}export function traverse():number{{{setup}let total=0;for(const k of {subject}){{const f=async ():Promise<number>=>await value(k);const p=f();total+=k;}}return total;}}export function fail():void{{throw new Error('stop');}}export function mutate():void{{m.forEach((v:number):void=>{{m.delete(v);}});}}export function size():number{{return m.size;}}"
        );
        let mut s = support::Script::new(&source);
        assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
        let (status, _, value) = s.call(s.index("traverse"), &[]);
        assert_eq!(status, ULUA_OK);
        assert_eq!(value.number, 1.0);
        let (status, code, _) = s.call(s.index("fail"), &[]);
        assert_eq!(
            (status, code),
            (ULUA_FAILED, "trap:uncaught-exception".into())
        );
        let (status, code, _) = s.call(s.index("mutate"), &[]);
        assert_eq!((status, code), (ULUA_FAILED, "trap:map-modified".into()));
        let (status, _, value) = s.call(s.index("size"), &[]);
        assert_eq!(status, ULUA_OK);
        assert_eq!(value.number, 0.0);
    }
}
