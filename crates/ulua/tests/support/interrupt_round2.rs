//! Review shapes place C15 operands and forward exits across interrupt boundaries.
use super::fill_at;

pub fn all(bytes: usize) -> Vec<(String, String)> {
    let parameters = (0..32)
        .map(|i| format!("p{i}:number"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["x"; 32].join(",");
    let before = "x++;".repeat(15);
    let function = format!("function g({parameters}):number{{return p0;}}");
    let head = "export function main(b:boolean):void{let x=1;";
    let mut shapes = vec![];
    for (name, prefix, statement) in [
        (
            "call-32",
            format!("function g({parameters}):void{{}}"),
            format!("g({arguments});"),
        ),
        (
            "new-32",
            format!("class K{{constructor({parameters}){{}}}}"),
            format!("new K({arguments});"),
        ),
        ("arrow-32", String::new(), format!("h({arguments});")),
        (
            "method-32",
            format!("class K{{m({parameters}):number{{return p0;}}}}"),
            format!("const k=new K();x=k.m({arguments});"),
        ),
        ("assign-call-32", function, format!("x=g({arguments});")),
        (
            "record-1024",
            String::new(),
            format!(
                "const o={{{}}};",
                (0..1024)
                    .map(|i| format!("a{i}:x"))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        ),
    ] {
        let arrow = if name == "arrow-32" {
            format!("const h=({parameters}):void=>{{}};")
        } else {
            String::new()
        };
        shapes.push((
            name.to_owned(),
            fill_at(
                bytes,
                &format!("{prefix}{head}{arrow}{before}{statement}}}"),
                " ",
                "",
            ),
        ));
        // Sweep every statement alignment. One alignment places the call after statement 15.
        for count in 0..16 {
            shapes.push((
                format!("{name}-boundary-{count}"),
                fill_at(
                    bytes,
                    &format!(
                        "{prefix}{head}{arrow}let c=false;{}{statement}}}",
                        "x++;".repeat(count)
                    ),
                    " ",
                    "",
                ),
            ));
        }
    }
    let seven = "x++;".repeat(7);
    let three = "x++;".repeat(3);
    let after = "x++;".repeat(20);
    // The declarations match the independent straight-path reference from the review.
    let head = "export function main(b:boolean):void{let x=1;let y=2;let s='';let c=false;";
    for (name, body) in [
        ("exit-straight", "x++;".repeat(40)),
        (
            "exit-break",
            format!("while(true){{{seven}if(b){{break;}}{three}}}{after}"),
        ),
        (
            "exit-continue",
            (0..16)
                .map(|pre| {
                    format!(
                        "for(let i=0;i<1;i++){{{}if(b){{continue;}}{three}}}{after}",
                        "x++;".repeat(pre)
                    )
                })
                .collect::<String>(),
        ),
        (
            "exit-condition",
            format!(
                "y=0;c=!c;c=!c;while((x<y)&&(x<y)){{{}}}{after}",
                "x++;".repeat(6)
            ),
        ),
        (
            "exit-nested",
            format!(
                "while(true){{while(true){{{seven}if(b){{break;}}{three}}}{three}if(b){{break;}}{three}}}{after}"
            ),
        ),
        (
            "exit-for-of",
            format!("for(const v of [1]){{{seven}if(b){{break;}}{three}}}{after}"),
        ),
    ] {
        shapes.push((
            name.to_owned(),
            fill_at(bytes, &format!("{head}{body}}}"), " ", ""),
        ));
        if name != "exit-straight" {
            // Earlier declarations can hide an exit defect at one statement alignment.
            for count in 0..16 {
                shapes.push((
                    format!("{name}-boundary-{count}"),
                    fill_at(
                        bytes,
                        &format!("{head}{}{body}}}", "x++;".repeat(count)),
                        " ",
                        "",
                    ),
                ));
            }
        }
    }
    for (name, body) in [
        ("async-prelude", String::new()),
        ("async-before-await", "x++;".repeat(48)),
        (
            "async-resume-1",
            format!("await tick();{}", "x++;".repeat(48)),
        ),
        (
            "async-resume-2",
            format!(
                "await tick();{}await tick();{}",
                "x++;".repeat(48),
                "x++;".repeat(48)
            ),
        ),
        (
            "async-reject-resume",
            format!("try{{await fail();}}catch(e){{{}}}", "x++;".repeat(48)),
        ),
    ] {
        let prefix = format!(
            "async function tick():Promise<number>{{return 1;}}async function fail():Promise<number>{{throw new Error('test');}}export async function main(b:boolean):Promise<number>{{let x=1;{body}return x;}}"
        );
        shapes.push((name.to_owned(), fill_at(bytes, &prefix, " ", "")));
    }
    shapes.push((
        "prelude-whitespace".into(),
        fill_at(
            bytes,
            "export function main(b:boolean):void{const s=b?'\\uFFFF':'\\uFFFE';const t=s.trim();}",
            " ",
            "",
        ),
    ));
    let arrows = (0..32)
        .flat_map(|value| std::iter::repeat_n(format!("():number=>{value}"), 2))
        .collect::<Vec<_>>()
        .join(",");
    shapes.push((
        "prelude-numeric-arrows".into(),
        fill_at(
            bytes,
            &format!("export function main(b:boolean):void{{const a=[{arrows}];}}"),
            " ",
            "",
        ),
    ));
    shapes
}
