//! Ceiling fixtures use short identifiers and dense declarations. No helper enters the library.
const FIRST: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_$";
const REST: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_$0123456789";
const RESERVED: &[&str] = &["do", "if", "in", "for", "let", "var", "new", "try"];

fn name(mut index: usize) -> String {
    let mut length = 1;
    let mut count = FIRST.len();
    while index >= count {
        index -= count;
        length += 1;
        count *= REST.len();
    }
    let mut bytes = vec![0; length];
    for byte in bytes.iter_mut().skip(1).rev() {
        *byte = REST[index % REST.len()];
        index /= REST.len();
    }
    bytes[0] = FIRST[index];
    bytes.into_iter().map(char::from).collect()
}

pub fn source(shape: &str, bytes: usize) -> String {
    if shape == "default-calls" {
        let prefix = format!(
            "function f({}):boolean{{return true;}}export function main():void{{",
            (0..32)
                .map(|i| format!("p{i}:boolean=true"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let count = (bytes - prefix.len() - 1) / 4;
        let mut source = format!("{prefix}{}}}", "f();".repeat(count));
        source.push_str(&" ".repeat(bytes - source.len()));
        return source;
    }

    let module = matches!(shape, "module-bindings" | "short-functions");
    let nested = matches!(shape, "nested-bindings" | "nested-writes");
    let writes = shape == "nested-writes";
    let prefix = if module {
        ""
    } else {
        "export function main():void{"
    };
    let suffix = if module {
        "export function main():void{}"
    } else {
        "}"
    };
    let make = |count: usize| {
        let names: Vec<_> = (0..)
            .map(name)
            .filter(|s| !RESERVED.contains(&s.as_str()))
            .take(count)
            .collect();
        let declarations = if shape == "short-functions" {
            names
                .iter()
                .map(|n| format!("function {n}():void{{}}"))
                .collect::<String>()
        } else {
            format!(
                "let {};",
                names
                    .iter()
                    .map(|n| format!("{n}=0"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let mut source = prefix.to_owned();
        if nested {
            source.push_str("let gate=true;");
        }
        if !nested || writes {
            source.push_str(&declarations);
        }
        if nested {
            // Assignment targets add one AST level beyond declaration initializers.
            source.push_str(&"if(gate)".repeat(if writes { 40 } else { 41 }));
            source.push('{');
            if writes {
                for n in &names {
                    source.push_str(&format!("{n}=1;"));
                }
            } else {
                source.push_str(&declarations);
            }
            source.push('}');
        }
        // These reads prove that all outer bindings remain live after every branch.
        if writes {
            for n in &names {
                source.push_str(&format!("{n}={n}+1;"));
            }
        }
        source.push_str(suffix);
        source
    };
    let (mut lo, mut hi) = (1, bytes / 4);
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if make(mid).len() <= bytes {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut source = make(lo);
    source.push_str(&" ".repeat(bytes.saturating_sub(source.len())));
    source
}
