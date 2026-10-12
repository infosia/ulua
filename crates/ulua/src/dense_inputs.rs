//! Dense P1, P2.1, and P2.2 repetitions for the output and heap measurements.
//! Each fixture puts its repeated form after 100 functions and inside a closure.
//! Each fixture uses the admitted depth of its context. Statements also cover empty bodies.

/// Every emitted construct and each distinct operator lowering has a dense fixture.
pub const SHAPES: &[&str] = &[
    "number",
    "boolean",
    "string",
    "parentheses",
    "negate",
    "not",
    "add",
    "subtract",
    "multiply",
    "divide",
    "remainder",
    "less",
    "greater",
    "less-equal",
    "greater-equal",
    "equal",
    "unequal",
    "loose-equal",
    "loose-unequal",
    "and",
    "or",
    "conditional",
    "template",
    "print",
    "String",
    "declare-let",
    "declare-const",
    "assign",
    "add-assign",
    "subtract-assign",
    "multiply-assign",
    "divide-assign",
    "remainder-assign",
    "increment",
    "decrement",
    "block",
    "if",
    "else-if",
    "while",
    "for",
    "break",
    "continue",
    "return",
    "function",
    "arrow-expression",
    "arrow-block",
    "call",
    "default-call",
    "function-type",
    "array-type",
    "array-literal",
    "array-nested",
    "array-read",
    "array-write",
    "array-add-write",
    "array-remainder-write",
    "array-length",
    "array-push",
    "for-of",
    "capture",
    "loop-capture",
    "block-empty",
    "if-empty",
    "while-empty",
    "for-empty",
    "return-empty",
    "parameters",
    "default-parameters",
    "arguments",
    "nested-functions",
    "array-review-add",
    "array-review-remainder",
    "array-review-read",
];

/// Repeat the smallest operation, with nesting or chaining where the form permits it.
pub fn source(shape: &str, bytes: usize) -> String {
    if shape.starts_with("array-review-") {
        let mut source = (0..100)
            .map(|i| format!("function f{i}():void{{}}"))
            .collect::<String>();
        let levels = 40;
        let read = shape == "array-review-read";
        source.push_str(&format!(
            "export function main():void{{const a={}1{};let i=0;{}",
            "[".repeat(levels),
            "]".repeat(levels),
            if read {
                "let x=0;"
            } else {
                "const h=():void=>{"
            }
        ));
        let index = format!("a{}", if read { "[0]" } else { "[i]" }.repeat(levels));
        let unit = match shape {
            "array-review-add" => format!("{index}+=i;"),
            "array-review-remainder" => format!("{index}%=i;"),
            _ => format!("x={index};"),
        };
        let suffix = if read { "}" } else { "};}" };
        let count = (bytes - source.len() - suffix.len()) / unit.len();
        source.push_str(&unit.repeat(count));
        source.push_str(suffix);
        source.push_str(&" ".repeat(bytes - source.len()));
        return source;
    }
    let mut prefix = (0..100)
        .map(|i| format!("function f{i}():void{{}}"))
        .collect::<String>();
    let levels = if shape == "array-read" { 39 } else { 40 };
    let nested = format!("{}1{}", "[".repeat(levels), "]".repeat(levels));
    prefix.push_str(&format!(
        "function q(x:number=0):number{{return x;}}export function main():void{{const a={nested};let i=0;const h=():void=>{{"
    ));
    // Spill reads use the longest storage path. Later binding ids also have four digits.
    for j in 0..200 {
        prefix.push_str(&format!("let v{j}=0;"));
    }
    prefix.push_str("let x=0,b=true,s='';const z=[1];let g=()=>0;");
    let chain = |term: &str, op: &str| vec![term; if term.len() > 1 { 38 } else { 40 }].join(op);
    let index = format!("a{}", "[i]".repeat(40));
    let (unit, suffix) = match shape {
        "number" => ("x=1;".into(), "};}"),
        "boolean" => ("b=true;".into(), "};}"),
        "string" => ("s='';".into(), "};}"),
        "parentheses" => (format!("x={}x{};", "(".repeat(38), ")".repeat(38)), "};}"),
        "negate" => (format!("x={}x;", "- ".repeat(38)), "};}"),
        "not" => (format!("b={}b;", "!".repeat(38)), "};}"),
        "add" | "subtract" | "multiply" | "divide" | "remainder" => {
            let op = match shape {
                "add" => "+",
                "subtract" => "-",
                "multiply" => "*",
                "divide" => "/",
                _ => "%",
            };
            (format!("x={};", chain("x", op)), "};}")
        }
        "less" | "greater" | "less-equal" | "greater-equal" | "equal" | "unequal"
        | "loose-equal" | "loose-unequal" => {
            let op = match shape {
                "less" => "<",
                "greater" => ">",
                "less-equal" => "<=",
                "greater-equal" => ">=",
                "equal" => "===",
                "unequal" => "!==",
                "loose-equal" => "==",
                _ => "!=",
            };
            (format!("b={};", chain(&format!("x{op}x"), "&&")), "};}")
        }
        "and" | "or" => (
            format!(
                "b={};",
                chain("b", if shape == "and" { "&&" } else { "||" })
            ),
            "};}",
        ),
        "conditional" => (format!("x={}x;", "b?x:".repeat(38)), "};}"),
        "template" => (format!("s=`{}`;", "${x}".repeat(64)), "};}"),
        "print" => ("print('');".into(), "};}"),
        "String" => ("s=String(x);".into(), "};}"),
        "declare-let" => ("{let t=0;}".into(), "};}"),
        "declare-const" => ("{const t=0;}".into(), "};}"),
        "assign" => ("x=x;".into(), "};}"),
        "add-assign" | "subtract-assign" | "multiply-assign" | "divide-assign"
        | "remainder-assign" => {
            let op = match shape {
                "add-assign" => "+",
                "subtract-assign" => "-",
                "multiply-assign" => "*",
                "divide-assign" => "/",
                _ => "%",
            };
            (format!("x{op}=x;"), "};}")
        }
        "increment" => ("x++;".into(), "};}"),
        "decrement" => ("x--;".into(), "};}"),
        "block" => (format!("{}x++;{}", "{".repeat(40), "}".repeat(40)), "};}"),
        "if" => (format!("{}x++;", "if(b)".repeat(40)), "};}"),
        "else-if" => (format!("{}x++;", "if(b){}else ".repeat(38)), "};}"),
        "while" => (format!("{}x++;", "while(b)".repeat(40)), "};}"),
        "for" => (format!("{}x++;", "for(;;)".repeat(34)), "};}"),
        "break" => ("while(b)break;".into(), "};}"),
        "continue" => ("while(b)continue;".into(), "};}"),
        "return" => ("if(b)return;".into(), "};}"),
        "function" => ("{function t():void{}}".into(), "};}"),
        "arrow-expression" => ("g=()=>0;".into(), "};}"),
        "arrow-block" => ("g=():number=>{return 0;};".into(), "};}"),
        "call" => (format!("x={}x{};", "q(".repeat(38), ")".repeat(38)), "};}"),
        "default-call" => ("q();".into(), "};}"),
        "function-type" => (
            format!("{{function t(f:{}number):void{{}}}}", "()=>".repeat(34)),
            "};}",
        ),
        "array-type" => (format!("{{const t:number{}=[];}}", "[]".repeat(38)), "};}"),
        "array-literal" => (format!("x=[{}][0];", vec!["0"; 125].join(",")), "};}"),
        "array-nested" => (
            format!("{{const t={}1{};}}", "[".repeat(38), "]".repeat(38)),
            "};}",
        ),
        "array-read" => (format!("x=a{};", "[i]".repeat(39)), "};}"),
        "array-write" => (format!("{index}=i;"), "};}"),
        "array-add-write" => (format!("{index}+=i;"), "};}"),
        "array-remainder-write" => (format!("{index}%=i;"), "};}"),
        "array-length" => ("x=z.length;".into(), "};}"),
        "array-push" => ("z.push(0);".into(), "};}"),
        "for-of" => (format!("{}x++;", "for(const t of z)".repeat(38)), "};}"),
        "capture" => ("g=()=>x;".into(), "};}"),
        "loop-capture" => ("for(const t of z)g=()=>t;".into(), "};}"),
        "block-empty" => (format!("{}{}", "{".repeat(40), "}".repeat(40)), "};}"),
        "if-empty" => (format!("{}{{}}", "if(b)".repeat(40)), "};}"),
        "while-empty" => (format!("{}{{}}", "while(b)".repeat(40)), "};}"),
        "for-empty" => (format!("{}{{}}", "for(;;)".repeat(34)), "};}"),
        "return-empty" => ("return;".into(), "};}"),
        "parameters" | "default-parameters" => {
            let parameters = (0..32)
                .map(|i| {
                    format!(
                        "p{i}:number{}",
                        if shape == "default-parameters" {
                            "=0"
                        } else {
                            ""
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            (format!("{{function t({parameters}):void{{}}}}"), "};}")
        }
        "arguments" => {
            prefix.push_str(&format!(
                "function t({}):void{{}}",
                (0..32)
                    .map(|i| format!("p{i}:number"))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            (format!("t({});", vec!["x"; 32].join(",")), "};}")
        }
        "nested-functions" => (
            format!("{{{}{}}}", "function t():void{".repeat(18), "}".repeat(18)),
            "};}",
        ),
        _ => unreachable!("known dense form"),
    };
    let group = match shape {
        // Each for node owns a control local. Captured blocks each need distinct storage.
        "for" | "for-empty" => 2,
        "function" | "function-type" | "loop-capture" | "parameters" | "default-parameters"
        | "nested-functions" => 64,
        _ => usize::MAX,
    };
    let mut source = prefix;
    let mut count = 0;
    let mut open = false;
    loop {
        let start = if count % group == 0 && group != usize::MAX {
            "(():void=>{"
        } else {
            ""
        };
        let end = if open && count % group == 0 {
            "})();"
        } else {
            ""
        };
        let reserve = if group == usize::MAX { 0 } else { 5 };
        if source.len() + end.len() + start.len() + unit.len() + reserve + suffix.len() > bytes {
            break;
        }
        source.push_str(end);
        source.push_str(start);
        source.push_str(&unit);
        open |= !start.is_empty();
        count += 1;
    }
    if open {
        source.push_str("})();");
    }
    source.push_str(suffix);
    source.push_str(&" ".repeat(bytes - source.len()));
    source
}
