//! Seed programs: the corpus, the families of the fixed-seed random checks, and the growth shapes.
use crate::rng::Rng;
use std::path::Path;

#[allow(dead_code, clippy::all)]
#[path = "../../ulua/tests/support/growth_shapes.rs"]
mod growth;

/// The seed programs of one run.
pub struct Pool {
    /// Each `corpus/**/*.ts` file, in path order.
    pub corpus: Vec<Vec<u8>>,
    shapes: Vec<growth::Shape>,
}

/// The host signature of each corpus fixture (corpus.md §7).
pub fn fixture(name: &str) -> Option<ulua_compiler::HostSignature> {
    let (p, r) = match name {
        "hostAdd" => (vec![1, 1], 1),
        "hostConcat" => (vec![3, 3], 3),
        "hostNot" => (vec![2], 2),
        "hostLog" => (vec![3], 0),
        "hostLater" => (vec![1], 1),
        _ => return None,
    };
    let mut s = ulua_compiler::HostSignature::new(name, p, r);
    s.asynchronous = name == "hostLater";
    Some(s)
}

/// The grants of a source: the fixtures of its `// grants:` header, or the host shapes
/// of the growth generators. The choice reads only the source, so a reduced input keeps it.
pub fn grants(source: &[u8]) -> Vec<ulua_compiler::HostSignature> {
    let text = String::from_utf8_lossy(source);
    let mut out: Vec<ulua_compiler::HostSignature> = vec![];
    let header = text
        .lines()
        .take_while(|l| l.trim().is_empty() || l.starts_with("//"))
        .find_map(|l| l.strip_prefix("// grants:"));
    for name in header.into_iter().flat_map(|h| h.split(',')) {
        if let Some(s) = fixture(name.trim())
            && !out.iter().any(|o| o.name == s.name)
        {
            out.push(s);
        }
    }
    if out.is_empty() {
        // The growth helpers read only the first line of a shape.
        out = std::panic::catch_unwind(|| growth::grants(&text)).unwrap_or_default();
    }
    out
}

impl Pool {
    /// Read the corpus under `root` and build the shape list.
    pub fn load(root: &Path) -> Result<Self, String> {
        let mut paths = vec![];
        for dir in ["accept", "reject", "trap"] {
            let d = root.join("corpus").join(dir);
            for e in std::fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))? {
                let p = e.map_err(|e| e.to_string())?.path();
                if p.extension().is_some_and(|x| x == "ts") {
                    paths.push(p);
                }
            }
        }
        paths.sort();
        let corpus = paths
            .iter()
            .map(|p| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display())))
            .collect::<Result<Vec<_>, _>>()?;
        if corpus.is_empty() {
            return Err("The corpus is empty".into());
        }
        // The ceiling keys of the loop shapes run 1,440 probe compiles; they stay out.
        let shapes = growth::all()
            .into_iter()
            .filter(|s| !(s.label.contains("loops") && s.key.starts_with("ceiling")))
            .collect();
        Ok(Self { corpus, shapes })
    }
    /// A pool from given programs, for tests.
    #[cfg(test)]
    pub fn from_programs(corpus: Vec<Vec<u8>>) -> Self {
        Self {
            corpus,
            shapes: vec![],
        }
    }
    /// One seed program: corpus, a random-check family, or a growth shape.
    pub fn base(&self, r: &mut Rng) -> Vec<u8> {
        match r.below(20) {
            0..=10 => r.pick(&self.corpus).clone(),
            11..=17 => generated(r).into_bytes(),
            _ => {
                let bytes = if r.one_in(16) {
                    r.range(48 * 1024, ulua_compiler::SOURCE_CEILING as usize + 512)
                } else {
                    r.range(200, 6000)
                };
                self.shape(r, bytes)
            }
        }
    }
    /// A growth shape of about `bytes` bytes.
    pub fn shape(&self, r: &mut Rng, bytes: usize) -> Vec<u8> {
        if self.shapes.is_empty() {
            return r.pick(&self.corpus).clone();
        }
        let s = r.pick(&self.shapes);
        let (make, key) = (s.make, s.key);
        // A generator can assert a minimum size; that input then comes from the corpus.
        match std::panic::catch_unwind(|| make(key, bytes)) {
            Ok(text) => text.into_bytes(),
            Err(_) => r.pick(&self.corpus).clone(),
        }
    }
}

/// One program of the random-check families (P2.7 exceptions, P4.1 async, P3 hosts).
pub fn generated(r: &mut Rng) -> String {
    match r.below(3) {
        0 => exceptions(r),
        1 => asynchronous(r),
        _ => hosts(r),
    }
}

const KINDS: [&str; 7] = [
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
    "EvalError",
    "URIError",
];

fn jump(r: &mut Rng, fallback: String) -> String {
    match r.below(5) {
        0 => fallback,
        1 => "throw e;".into(),
        2 => "break;".into(),
        3 => "continue;".into(),
        _ => "score+=2;".into(),
    }
}

/// The P2.7 random family: protected loops with every completion kind.
fn exceptions(r: &mut Rng) -> String {
    let kind = r.pick(&KINDS);
    let tested = r.pick(&KINDS);
    let n = r.below(9);
    let pending = jump(r, format!("return {n};")).replace("throw e;", "throw err;");
    let caught = jump(r, format!("return {};", n + 10));
    let finalizer = match r.below(5) {
        0 => format!("return {};", n + 20),
        1 => "throw new URIError('final');".into(),
        2 => "break;".into(),
        3 => "continue;".into(),
        _ => "score+=3;".into(),
    };
    let inner = format!(
        "try{{print(`try${{k}}`);{pending}}}catch(e){{if(e instanceof {tested}){{print(e.message);}}{caught}}}finally{{print('final');{finalizer}}}"
    );
    let body = if r.one_in(2) {
        format!("try{{{inner}}}finally{{print('outer');}}")
    } else {
        inner
    };
    format!(
        "let score:number=0;function f():number{{const err=new {kind}('m{n}');err.name='custom';for(let k=0;k<3;k++){{{body}score+=5;}}return score;}}export function main():void{{try{{print(`${{f()}}`);}}catch(e){{if(e instanceof Error){{print(`${{e.name}}: ${{e.message}}`);}}else{{throw e;}}}}print(`${{score}}`);}}"
    )
}

/// The P4.1 random family: async tasks with host requests and derived callbacks.
fn asynchronous(r: &mut Rng) -> String {
    let mut functions = String::from(
        "// grants: hostLater\nasync function value(n:number):Promise<number>{return n;}async function failure(n:number):Promise<number>{await value(n);throw new RangeError('bad');}",
    );
    for task in 0..3 {
        let count = 1 + r.below(4);
        let source = if r.one_in(3) {
            "failure"
        } else if r.one_in(2) {
            "hostLater"
        } else {
            "value"
        };
        let pending = *r.pick(&[
            "return await value(sum);",
            "return value(sum);",
            "throw new TypeError('return');",
            "sum+=await hostLater(1);",
        ]);
        let caught = *r.pick(&[
            "return await value(9);",
            "throw e;",
            "return hostLater(11);",
            "sum+=await value(3);",
        ]);
        let finalizer = *r.pick(&[
            "return await value(13);",
            "return value(17);",
            "throw new URIError('final');",
            "sum+=await hostLater(5);",
        ]);
        functions.push_str(&format!("async function task{task}():Promise<number>{{let sum:number=0;print('t{task} start');for(let k=0;k<{count};k++){{try{{sum+=await {source}(k);print(`t{task} value ${{sum}}`);{pending}}}catch(e){{print('t{task} catch');if(e instanceof Error){{print(e.name);}}{caught}}}finally{{await value(k);print('t{task} finally');{finalizer}}}}}return sum;}}"));
    }
    let mut body =
        String::from("const p0=task0();const p1=task1();const p2=task2();print('started');");
    for task in if r.one_in(2) { [0, 1, 2] } else { [2, 0, 1] } {
        body.push_str(&format!("try{{print(`p{task} ${{await p{task}}} ${{await p{task}}}`);}}catch(e){{if(e instanceof Error){{print(`p{task} ${{e.name}}`);}}}}"));
    }
    if r.one_in(8) {
        body.push_str("const lost=failure(1);");
    }
    format!("{functions}export async function main():Promise<void>{{{body}}}")
}

/// The P3 random family: every sync fixture at every call position.
fn hosts(r: &mut Rng) -> String {
    let a = r.below(41);
    let b = r.below(13);
    let flag = if r.one_in(2) { "true" } else { "false" };
    let text = *r.pick(&["é", "🙂", "", "x\\0y"]);
    let index = r.below(1000);
    format!(
        r#"// grants: hostAdd, hostConcat, hostNot, hostLog
let n:number={a};
let base:number=hostAdd({a},{b});
let word:string=hostConcat('{text}','{index}');
let flag:boolean=hostNot({flag});
function next():number{{n=hostAdd(n,1);hostLog(`${{n}}`);return n;}}
function numberDefault(x:number=hostAdd(next(),next())):number{{return hostAdd(x,{b});}}
function stringDefault(s:string=hostConcat(word,'!')):string{{return hostConcat(s,'?');}}
class Box{{n:number=hostAdd(base,1);s:string=hostConcat(word,'c');b:boolean=hostNot(flag);
method():string{{hostLog(this.s);this.n=hostAdd(this.n,2);this.b=hostNot(this.b);return hostConcat(this.s,`${{this.n}} ${{this.b}}`);}}}}
export function main():void{{
const numberArrow=(x:number):number=>hostAdd(x,{b});
print(`${{hostAdd(next(),next())}}`);print(`${{numberDefault()}}`);print(stringDefault());
const object=new Box();print(object.method());print(`${{numberArrow(base)}}`);
const xs:number[]=[1,2,3];print(xs.map((x:number):number=>hostAdd(x,{a})).join(','));
try{{hostLog(hostConcat(word,'try'));throw new Error(word);}}
catch(e){{if(e instanceof Error){{hostLog(hostConcat(e.message,'catch'));}}}}
finally{{hostLog(hostConcat(word,'finally'));}}
print(`${{n}}`);
}}
export function take(x:number,s:string,b:boolean):string{{return hostConcat(s,`${{hostAdd(x,{a})}} ${{hostNot(b)}}`);}}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_families_compile() {
        let mut r = Rng::new(5);
        for _ in 0..30 {
            let s = generated(&mut r);
            let g = grants(s.as_bytes());
            let result = ulua_compiler::compile_with_grants(
                s.as_bytes(),
                ulua_compiler::Limits::default(),
                &g,
            );
            assert!(result.is_ok(), "{s}: {:?}", result.err());
        }
    }
    #[test]
    fn grants_read_the_header() {
        let g = grants(b"// grants: hostAdd, hostLater, nope\nexport function main():void{}");
        assert_eq!(g.len(), 2);
        assert!(g[1].asynchronous);
    }
}
