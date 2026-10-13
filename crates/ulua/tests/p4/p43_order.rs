//! Six hand-pinned request orders and four reaction orders use the one-settle node loop.
//! Costs include one tsc batch, one analyzer batch, and fourteen ABI runs.
#[allow(dead_code)]
#[path = "../support/script.rs"]
mod support;
#[allow(dead_code)]
#[path = "../support/equality.rs"]
mod witnesses;
use std::time::Instant;
#[test]
fn host_request_job_order_matches_node() {
    let now = Instant::now();
    let mut suite = witnesses::Suite::named("P4.3 order");
    let cases = [
        (
            "export async function main():Promise<void>{print('start');print(`${await hostLater(1)}`);print(`${await hostLater(2)}`);print('end');}",
            Some("start\n1\n2\nend\n"),
        ),
        (
            "async function task(n:number):Promise<number>{print(`start ${n}`);const a=await hostLater(n);print(`first ${a}`);const b=await hostLater(n+1);print(`second ${b}`);return a+b;}export async function main():Promise<void>{const a=task(1);const b=task(10);print('wait');print(`${await a},${await b}`);}",
            Some("start 1\nstart 10\nwait\nfirst 1\nfirst 10\nsecond 2\nsecond 11\n3,21\n"),
        ),
        (
            "export async function main():Promise<void>{const a=hostLater(1).then((n:number):number=>{print(`a ${n}`);return n;});const b=hostLater(2).then((n:number):number=>{print(`b ${n}`);return n;});print('wait');const ns=await Promise.all([b,a]);print(`${ns[0]},${ns[1]}`);}",
            Some("wait\na 1\nb 2\n2,1\n"),
        ),
        (
            "export async function main():Promise<void>{const p=hostLater(1).then((n:number):Promise<number>=>{print(`then ${n}`);return hostLater(n+1);});const q=hostLater(3).then((n:number):void=>print(`other ${n}`));print(`${await p}`);await q;}",
            Some("then 1\nother 3\n2\n"),
        ),
        (
            "async function done():Promise<void>{print('finally');await hostLater(9);print('done');}export async function main():Promise<void>{const p=hostLater(1).finally(done);const q=hostLater(2).then((n:number):void=>print(`other ${n}`));print(`${await p}`);await q;}",
            Some("finally\nother 2\ndone\n1\n"),
        ),
        (
            "export async function main():Promise<void>{const a=hostLater(1);const b=a.then((n:number):number=>{print('one');return n;});const c=a.then((n:number):number=>{print('two');return n;});await a;print('await');await b;await c;}",
            Some("one\ntwo\nawait\n"),
        ),
        (
            "async function next(n:number):Promise<number>{await hostLater(n);throw new TypeError('bad');}export async function main():Promise<void>{const p=hostLater(1).then(next).catch((e:unknown):Promise<number>=>{print('catch');return hostLater(2);}).finally(():void=>print('finally'));print(`${await p}`);}",
            None,
        ),
        (
            "async function task(n:number):Promise<number>{await hostLater(n);if(n===1)throw new RangeError('first');return n;}export async function main():Promise<void>{try{const ns=await Promise.all([task(1),task(2)]);print(`${ns.length}`);}catch{print('caught');}}",
            None,
        ),
        (
            "export async function main():Promise<void>{const ns=[1,2,3];const ps=ns.map(async(n:number):Promise<number>=>{const a=await hostLater(n);const b=await hostLater(n+10);return a+b;});const xs=await Promise.all(ps);print(`${xs[0]},${xs[1]},${xs[2]}`);}",
            None,
        ),
        (
            "export async function main():Promise<void>{const p=hostLater(1).finally(():void=>{throw new URIError('final');});try{await p;}catch(e){if(e instanceof Error)print(e.name);}print(`${await hostLater(2)}`);}",
            None,
        ),
    ];
    for (source, golden) in cases {
        suite.source(&format!("// grants: hostLater\n{source}"), golden);
    }
    for prefix in ["", "await hostLater(1);"] {
        let source = format!(
            "// grants: hostLater\nexport async function main():Promise<void>{{{prefix}const p=hostLater(2).then((n:number):void=>print('late'));throw new RangeError('main');}}"
        );
        suite.source_async_trap(&source, "RangeError");
    }
    suite.source_trap("// grants: hostLater\nasync function tick():Promise<number>{return 1;}class C{p:Promise<number>=tick();async wait():Promise<number>{await hostLater(1);return await this.p;}}export async function main():Promise<void>{const c=new C();c.p=c.wait();await c.p;}", "", "async:pending", "Error");
    suite.source_async_trap("// grants: hostLater\nexport async function main():Promise<void>{const p=hostLater(1);const lost=p.then((n:number):number=>{throw new TypeError('lost');});await p;const later=hostLater(2).then((n:number):void=>print('late'));throw new RangeError('main');}", "RangeError");
    suite.finish();
    eprintln!(
        "P4.3 direct order programs={} golden=6 cost={:.3}s",
        suite.count,
        now.elapsed().as_secs_f64()
    );
}
