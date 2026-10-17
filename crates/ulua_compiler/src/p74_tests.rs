use crate::{Limits, compile};
fn check(body: &str) -> Result<crate::Compiled, crate::Diagnostic> {
    compile(
        format!("export function main():void{{{body}}}").as_bytes(),
        Limits::default(),
    )
}
#[test]
fn ordered_collection_corpus_and_rejections() {
    for source in [
        include_str!("../../../corpus/accept/a77-map-basics.ts"),
        include_str!("../../../corpus/accept/a78-map-order.ts"),
        include_str!("../../../corpus/accept/a79-set-basics.ts"),
        include_str!("../../../corpus/trap/t13-map-modified.ts"),
    ] {
        compile(source.as_bytes(), Limits::default()).unwrap();
    }
    for (body, code, message) in [
        (
            "new Map<()=>number,number>();",
            "C6",
            "a Map or Set key type has a defined equality",
        ),
        (
            "new Set<number|null>();",
            "C6",
            "a Map or Set key type has a defined equality",
        ),
        (
            "new Set<Promise<number>>();",
            "C6",
            "a Map or Set key type has a defined equality",
        ),
        (
            "const m=new Map<number,number>();m.entries();",
            "C6",
            "entries needs a tuple type",
        ),
        (
            "const m=new Map<number,number>();for(const x of m){}",
            "C6",
            "for...of over a Map needs a tuple type; iterate keys() or values()",
        ),
        (
            "const m=new Map<number,number|null>();m.get(1);",
            "C2",
            "get needs a value type without null; use has and getOr",
        ),
    ] {
        let e = check(body).expect_err(body);
        assert_eq!((e.code, e.message.as_str()), (code, message), "{body}");
    }
}
#[test]
fn ordered_collection_closed_surface() {
    for body in [
        "const m=new Map<number,number>();const x=m.keys();",
        "const m=new Map<number,number>();const x=m.values();",
        "const m=new Map<number,number>();const x=m.get;",
        "const m=new Map<number,number>();m.forEach((v:number,k:number,m:Map<number,number>):void=>{});",
        "const s=new Set<number>();s.forEach((v:number,k:number):void=>{});",
        "const s=new Set<number>();s.union(s);",
        "const x=Map;",
        "const x=Set;",
        "Map.groupBy([1],(v:number):number=>v);",
        "new WeakMap();",
        "new WeakSet();",
        "new Map<number,number>([[1,2]]);",
    ] {
        assert!(check(body).is_err(), "{body}");
    }
    for body in [
        "const m:Map<number,string>=new Map();m.set(1,'a');",
        "const m=new Map<number,null>();m.set(1,null);const n=m.getOr(1,null);",
        "const s:Set<number>=new Set([1,2]);",
        "const m=new Map<number,number|null>();m.set(1,null);const x=m.getOr(1,null);",
        "const m=new Map<number,string>();const x=m.get(1);if(x!=null){print(x);}",
    ] {
        check(body).unwrap();
    }
}

#[test]
fn collection_internal_storage_and_context_do_not_change_source_names() {
    for body in [
        "const s=new Set<number>([1]);for(const __ulua_collection of s){const n:number=__ulua_collection;}",
        "const __ulua_collection=7;const s=new Set<number>([1]);for(const k of s){const n:number=__ulua_collection;}",
        "const m:Map<number,string>|null=new Map();",
        "const s=new Set<number>(([]));",
    ] {
        check(body).unwrap();
    }
    for source in [
        "function f(m:Map<number,string>):void{}export function main():void{}",
        "type R={x:number};function f(s:Set<R>):void{}export function main():void{}",
    ] {
        let result = compile(source.as_bytes(), Limits::default()).unwrap();
        assert!(result.luau.contains("type __ulua_Table"));
    }
}

#[test]
fn collection_nullable_optional_access_and_source_shadowing() {
    for source in [
        "export function main():void{const m:Map<number,number>|null=null;m?.set(1,2);const n=m?.size??0;}",
        "export function main():void{const s:Set<number>|null=null;s?.add(1);const n=s?.size??0;}",
        "class Map{x:number=1;}export function main():void{const m:Map=new Map();print(`${m.x}`);}",
        "class Set{x:number=1;}export function main():void{const s:Set=new Set();print(`${s.x}`);}",
        "type Map={x:number};export function main():void{const m:Map={x:1};}",
    ] {
        compile(source.as_bytes(), Limits::default()).unwrap();
    }
    let error = check("new Map<number,number>([[1,2]]);").unwrap_err();
    assert_eq!(error.code, "C6");
}

#[test]
fn collection_traversal_rejects_await_at_each_body_position() {
    for subject in ["m.keys()", "m.values()", "s"] {
        for body in [
            "await value(k);",
            "{{await value(k);}}",
            "if(k>0){await value(k);}",
            "while(k>0){await value(k);break;}",
            "for(let i=0;i<1;i++){await value(k);}",
            "for(const x of [k]){await value(x);}",
            "for(const x of m.values()){await value(x);}",
            "try{await value(k);}catch(e){}",
            "try{}finally{await value(k);}",
            "const n=await value(k);",
            "for(let i=await value(k);i<1;i++){}",
            "for(let i=0;i<await value(k);i++){}",
            "for(let i=0;i<1;i+=await value(k)){}",
            "for(const x of [await value(k)]){}",
            "return await value(k);",
        ] {
            let source = format!(
                "async function value(n:number):Promise<number>{{return n;}}export async function main():Promise<number>{{const m=new Map<number,number>();const s=new Set<number>();for(const k of {subject}){{{body}}}return 0;}}"
            );
            let error = compile(source.as_bytes(), Limits::default()).unwrap_err();
            assert_eq!(error.code, "C6", "{source}");
            assert_eq!(
                error.message, "await in a Map or Set traversal is rejected",
                "{source}"
            );
        }
    }
}

#[test]
fn collection_for_each_rejects_promise_callbacks() {
    for collection in ["new Map<number,number>()", "new Set<number>()"] {
        for callback in [
            "async (v:number):Promise<number>=>await value(v)",
            "(async (v:number):Promise<number>=>{return await value(v);})",
            "value",
            "(v:number):Promise<number>=>value(v)",
            "callback",
        ] {
            let source = format!(
                "async function value(n:number):Promise<number>{{return n;}}const callback=value;export function main():void{{const m={collection};m.forEach({callback});}}"
            );
            let error = compile(source.as_bytes(), Limits::default()).unwrap_err();
            assert_eq!(error.code, "C6", "{source}");
            assert_eq!(
                error.message, "await in a Map or Set traversal is rejected",
                "{source}"
            );
        }
    }
}

#[test]
fn collection_traversal_allows_await_in_a_separate_function_frame() {
    for body in [
        "const f=async ():Promise<number>=>await value(k);",
        "async function f():Promise<number>{return await value(k);}",
        "const f=async ():Promise<number>=>{return await value(k);};const p=f();",
    ] {
        let source = format!(
            "async function value(n:number):Promise<number>{{return n;}}export function main():void{{const s=new Set<number>([1]);for(const k of s){{{body}}}}}"
        );
        compile(source.as_bytes(), Limits::default()).unwrap();
    }
}
