//! Direct grant compiler API, C18 forms, and normal script shadowing.
use crate::*;
fn grant() -> HostSignature {
    HostSignature::new("host", vec![1, 2, 3], 1)
}
#[test]
fn host_signature_and_grant_compiler_api() {
    let g = grant();
    assert_eq!(g.name, "host");
    assert_eq!(g.parameters, [1, 2, 3]);
    assert_eq!(g.result, 1);
    let source = b"export function main():void{const n=host(1,true,'x');}";
    let c = compile_with_grants(source, Limits::default(), &[g]).unwrap();
    assert_eq!(c.hosts, ["host"]);
    assert!(c.luau.contains("__ulua_grants[1]"));
    assert_eq!(compile(source, Limits::default()).unwrap_err().code, "C18");
    let c = compile_with_grants(
        b"export function main():void{}",
        Limits::default(),
        &[grant()],
    )
    .unwrap();
    assert!(c.hosts.is_empty());
    assert!(!c.luau.contains("__ulua_grants"));
}
#[test]
fn every_host_value_use_is_c18() {
    for body in [
        "const f=host;",
        "const f=host as (a:number,b:boolean,c:string)=>number;",
        "const f=host satisfies (a:number,b:boolean,c:string)=>number;",
        "const f=host!;",
        "(host)(1,true,'x');",
        "host=host;",
        "print(`${host===host}`);",
        "print(`${typeof host}`);",
        "print(`${typeof (host)}`);",
        "const xs=[host];",
        "const x={f:host};",
        "function f(x:(a:number,b:boolean,c:string)=>number):void{}f(host);",
        "host?.(1,true,'x');",
    ] {
        let source = format!("export function main():void{{{body}}}");
        assert_eq!(
            compile_with_grants(source.as_bytes(), Limits::default(), &[grant()])
                .unwrap_err()
                .code,
            "C18",
            "{body}"
        );
    }
    let source = b"export {host};export function main():void{}";
    assert_eq!(
        compile_with_grants(source, Limits::default(), &[grant()])
            .unwrap_err()
            .code,
        "C18"
    );
}
#[test]
fn script_shadowing_uses_script_types_and_unused_grants_have_no_closure() {
    for body in [
        "const host=1;print(`${host}`);",
        "function f(host:number):void{print(`${host}`);}f(1);",
        "const f=(host:number):number=>host;print(`${f(1)}`);",
        "try{}catch(host){}",
        "{let host=1;print(`${host}`);}",
        "function host():number{return 1;}print(`${host()}`);",
    ] {
        let source = format!("export function main():void{{{body}}}");
        let c = compile_with_grants(source.as_bytes(), Limits::default(), &[grant()]).unwrap();
        assert!(c.hosts.is_empty(), "{body}");
    }
    let c = compile_with_grants(
        b"const host=():number=>42;export function main():void{print(`${host()}`);}",
        Limits::default(),
        &[grant()],
    )
    .unwrap();
    assert!(c.hosts.is_empty());
}
#[test]
fn every_declare_form_is_c18() {
    for declaration in [
        "declare function host(a:number,b:boolean,c:string):number;",
        "declare const host:number;",
        "declare let host:number;",
        "declare class Host{}",
        "class Host{declare n:number;}",
        "declare namespace Host{}",
        "declare module 'host'{}",
        "declare global{}",
        "export declare function host(a:number,b:boolean,c:string):number;",
        "export declare const host:number;",
        "declare type T={a:number};",
        "declare interface T{}",
        "declare enum E{A}",
    ] {
        let source = format!("{declaration}export function main():void{{}}");
        assert_eq!(
            compile_with_grants(source.as_bytes(), Limits::default(), &[grant()])
                .unwrap_err()
                .code,
            "C18",
            "{declaration}"
        );
    }
}
#[test]
fn native_closures_follow_grant_order_and_grants_have_no_tdz() {
    let grants = [
        HostSignature::new("second", vec![], 1),
        HostSignature::new("first", vec![], 1),
    ];
    let c=compile_with_grants(b"function f():number{return first()+second();}export function main():void{print(`${f()}`);}",Limits::default(),&grants).unwrap();
    assert_eq!(c.hosts, ["second", "first"]);
}

#[test]
fn library_names_outside_the_surface_are_c11() {
    for body in [
        "Number('1');",
        "JSON.stringify(1);",
        "parseInt('1');",
        "isNaN(1);",
        "Date.now();",
        "const x=globalThis;",
        "Boolean(1);",
        "Array.isArray([1]);",
    ] {
        let source = format!("export function main():void{{{body}}}");
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C11",
            "{body}"
        );
    }
    for body in ["missing();", "const x=missing;", "missing.method();"] {
        let source = format!("export function main():void{{{body}}}");
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C18",
            "{body}"
        );
    }
}

#[test]
fn namespaces_and_modules_without_declare_are_c11() {
    for form in [
        "namespace N{}",
        "module M{}",
        "export namespace N{}",
        "export module M{}",
    ] {
        let source = format!("{form}export function main():void{{}}");
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C11",
            "{form}"
        );
    }
}

#[test]
fn host_construction_states_the_call_only_rule() {
    let source = b"export function main():void{new hostAdd(1,2);}";
    let diagnostic = compile_with_grants(
        source,
        Limits::default(),
        &[HostSignature::new("hostAdd", vec![1, 1], 1)],
    )
    .unwrap_err();
    assert_eq!(diagnostic.code, "C18");
    assert_eq!(diagnostic.message, "A host function is only called");
}
