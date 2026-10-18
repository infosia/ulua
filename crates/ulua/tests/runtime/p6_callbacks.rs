//! Hand values cover inline exception exits and receiver snapshots.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
use ulua::*;

#[test]
fn inline_callbacks_preserve_exception_and_initial_value_order() {
    // Base 100 encodes kept [1, 50, 50], then xs [99, 99, 99], in the filter case.
    for (source, expected) in [
        (
            "let seen=0;function visit(v:number):void{seen=seen*10+v;if(v===2){throw new Error('stop');}}export function main():number{const a=[1,2,3];try{a.forEach((v:number)=>visit(v));}catch(e){}return seen;}",
            12.0,
        ),
        (
            "export function main():number{let a=[1,2];const original=a;const init=():number=>{original.push(3);a=[9];return 10;};const result=a.reduce((acc:number,v:number)=>acc+v,init());return result*10+a[0];}",
            169.0,
        ),
        (
            "export function main():number{const a=[1,2];const result=a.map((v:number)=>a.map((w:number)=>v+w)[0]);return result[0]*10+result[1];}",
            23.0,
        ),
        (
            "export function main():number{const xs=[1,2,3];const change=(i:number):boolean=>{xs[i]=99;if(i+1<xs.length){xs[i+1]=50;}return true;};const kept=xs.filter((v:number,i:number)=>change(i));let result=0;for(const v of kept){result=result*100+v;}for(const v of xs){result=result*100+v;}return result;}",
            15050999999.0,
        ),
    ] {
        let mut script = support::Script::new(source);
        assert_eq!(script.instance(RunLimits::default()).0, ULUA_OK);
        let (status, code, value) = script.call(script.index("main"), &[]);
        assert_eq!(status, ULUA_OK, "{code}: {source}");
        assert_eq!(value.number, expected, "{source}");
    }
}
