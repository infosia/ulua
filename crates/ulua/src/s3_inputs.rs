//! Shared P1 measurement and boundary sources. These helpers enter no product build.
pub const FORMS: &[&str] = &[
    "parentheses",
    "not",
    "minus",
    "conditional",
    "if",
    "else-if",
    "block",
    "while",
    "for",
    "call",
    "flat",
    "template",
    "assignment",
];

pub fn nested(form: &str, levels: usize) -> String {
    let mut expr = if form == "not" { "true" } else { "1" }.to_owned();
    let body;
    match form {
        "parentheses" | "not" | "minus" | "conditional" | "call" | "template" => {
            for _ in 0..levels {
                expr = match form {
                    "parentheses" => format!("({expr})"),
                    "not" => format!("!{expr}"),
                    "minus" => format!("- {expr}"),
                    "conditional" => format!("true ? 1 : {expr}"),
                    "call" => format!("f({expr})"),
                    _ => format!("`x${{{expr}}}`"),
                };
            }
            body = format!("const a = {expr};");
        }
        "flat" => body = format!("const a = 1{};", "+1".repeat(levels)),
        "assignment" => body = format!("let a = 0; a = {}1;", "a = ".repeat(levels)),
        "else-if" => body = format!("{}{{}}", "if (false) {} else ".repeat(levels)),
        "if" | "block" | "while" | "for" => {
            let prefix = match form {
                "if" => "if (true) {",
                "block" => "{",
                "while" => "while (false) {",
                _ => "for (;false;) {",
            };
            body = format!("{}{}", prefix.repeat(levels), "}".repeat(levels));
        }
        _ => unreachable!("known measurement form"),
    }
    format!("function f(x:number):number {{return x;}} export function main():void {{{body}}}")
}

pub fn parameters(count: usize, defaults: bool, call: bool) -> String {
    let params = (0..count)
        .map(|i| format!("p{i}:number{}", if defaults { "=1" } else { "" }))
        .collect::<Vec<_>>()
        .join(",");
    let body = if call {
        format!("f({});", vec!["1"; count].join(","))
    } else {
        String::new()
    };
    format!("function f({params}):void {{}} export function main():void {{{body}}}")
}

pub fn template(substitutions: usize) -> String {
    format!(
        "export function main():void {{const a = `{}x`;}}",
        "x${1}".repeat(substitutions)
    )
}

pub fn loops(count: usize) -> String {
    format!(
        "export function main():void {{{}}}",
        "for (;false;) {}".repeat(count)
    )
}

pub fn bindings(count: usize) -> String {
    let body = (0..count)
        .map(|i| format!("const a{i}=1;"))
        .collect::<String>();
    format!("export function main():void {{{body}}}")
}

pub fn recursion() -> &'static str {
    "export function recurse(n:number):number {if(n===0){return 1;}return recurse(n-1);} export function main():void {}"
}

pub fn source_ceiling(kind: &str, bytes: usize) -> String {
    let mut s = if kind == "functions" || kind == "parameter-functions" {
        "export function main():void {}".to_owned()
    } else {
        "export function main():void {".to_owned()
    };
    let end = if kind == "functions" || kind == "parameter-functions" {
        ""
    } else {
        "}"
    };
    let mut i = 0;
    loop {
        let unit = match kind {
            "functions" => format!("function f{i}():void {{}}"),
            "parameter-functions" => format!(
                "function f{i}({}):void {{}}",
                (0..32)
                    .map(|p| format!("p{p}:number"))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            "expressions" => format!("const a{i}=1+2+3+4+5+6+7+8;"),
            "templates" => format!("const a{i}=`x${{1}}x${{1}}x${{1}}`;"),
            "unary" => format!("const a{i}={}true;", "!".repeat(42)),
            "flat" => format!("const a{i}=1{};", "+1".repeat(42)),
            "dense" => format!("let a{i}=1;"),
            _ => format!("const a{i}=1;"),
        };
        if s.len() + unit.len() + end.len() > bytes {
            break;
        }
        s.push_str(&unit);
        i += 1;
    }
    s.push_str(end);
    s.push_str(&" ".repeat(bytes.saturating_sub(s.len())));
    s
}
