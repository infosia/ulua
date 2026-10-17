//! Closed string member signatures and helper names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum StringMethod {
    Slice,
    Substring,
    Substr,
    IndexOf,
    LastIndexOf,
    Includes,
    StartsWith,
    EndsWith,
    CharAt,
    CharCodeAt,
    CodePointAt,
    At,
    Split,
    Trim,
    TrimStart,
    TrimEnd,
    Repeat,
    PadStart,
    PadEnd,
    Replace,
    ReplaceAll,
    Concat,
    Next,
    Upper,
    Lower,
}
impl StringMethod {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "toUpperCase" => Self::Upper,
            "toLowerCase" => Self::Lower,
            "slice" => Self::Slice,
            "substring" => Self::Substring,
            "substr" => Self::Substr,
            "indexOf" => Self::IndexOf,
            "lastIndexOf" => Self::LastIndexOf,
            "includes" => Self::Includes,
            "startsWith" => Self::StartsWith,
            "endsWith" => Self::EndsWith,
            "charAt" => Self::CharAt,
            "charCodeAt" => Self::CharCodeAt,
            "codePointAt" => Self::CodePointAt,
            "at" => Self::At,
            "split" => Self::Split,
            "trim" => Self::Trim,
            "trimStart" | "trimLeft" => Self::TrimStart,
            "trimEnd" | "trimRight" => Self::TrimEnd,
            "repeat" => Self::Repeat,
            "padStart" => Self::PadStart,
            "padEnd" => Self::PadEnd,
            "replace" => Self::Replace,
            "replaceAll" => Self::ReplaceAll,
            "concat" => Self::Concat,
            _ => return None,
        })
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Upper => "str_upper",
            Self::Lower => "str_lower",
            Self::Slice => "str_slice",
            Self::Substring => "str_substring",
            Self::Substr => "str_substr",
            Self::IndexOf => "str_indexOf",
            Self::LastIndexOf => "str_lastIndexOf",
            Self::Includes => "str_includes",
            Self::StartsWith => "str_startsWith",
            Self::EndsWith => "str_endsWith",
            Self::CharAt => "str_charAt",
            Self::CharCodeAt => "str_charCodeAt",
            Self::CodePointAt => "str_codePointAt",
            Self::At => "str_at",
            Self::Split => "str_split",
            Self::Trim => "str_trim",
            Self::TrimStart => "str_trimStart",
            Self::TrimEnd => "str_trimEnd",
            Self::Repeat => "str_repeat",
            Self::PadStart => "str_padStart",
            Self::PadEnd => "str_padEnd",
            Self::Replace => "str_replace",
            Self::ReplaceAll => "str_replaceAll",
            Self::Concat => "str_concat",
            Self::Next => "str_next",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Limits, compile};

    #[test]
    fn case_and_scalar_calls_have_native_lowerings() {
        for (expression, helper) in [
            ("'Straße'.toUpperCase()", "case_upper"),
            ("'ΟΣ'.toLowerCase()", "case_lower"),
            ("String.fromCodePoint(128512)", "case_fromCodePoint"),
            ("String.fromCharCode(65)", "case_fromCharCode"),
        ] {
            let source = format!("export function main():string{{return {expression};}}");
            let compiled = compile(source.as_bytes(), Limits::default()).unwrap();
            assert!(compiled.native_math);
            assert!(
                compiled
                    .luau
                    .contains(&format!("__ulua_math_native.{helper}"))
            );
        }
    }
    #[test]
    fn case_diagnostics_name_arity_and_locale() {
        for name in ["fromCodePoint", "fromCharCode"] {
            for args in ["", "65,66"] {
                let source = format!("export function main():void{{String.{name}({args});}}");
                let error = compile(source.as_bytes(), Limits::default()).unwrap_err();
                assert_eq!(error.code, "C5");
                assert_eq!(
                    error.message,
                    format!("String.{name} takes exactly one argument")
                );
            }
        }
        for name in ["toLocaleUpperCase", "toLocaleLowerCase", "localeCompare"] {
            let source = format!("export function main():void{{'x'.{name}();}}");
            let error = compile(source.as_bytes(), Limits::default()).unwrap_err();
            assert_eq!(error.code, "C5");
            assert_eq!(error.message, format!("{name} depends on locale data"));
        }
    }

    #[test]
    fn string_surface_rejects_unlisted_members_and_forms() {
        for body in [
            "const c='x'[0];",
            "const x='a'<'b';",
            "const f='x'.slice;",
            "'x'.toLocaleUpperCase();",
            "'x'.toLocaleLowerCase();",
            "'x'.localeCompare('y');",
            "'x'.normalize();",
            "'x'.match('x');",
            "'x'.matchAll('x');",
            "'x'.search('x');",
            "'x'.replace(/x/,'y');",
            "'x'.replaceAll(/x/g,'y');",
            "String.raw('x');",
            "const f=String.fromCharCode;",
            "const f=String.fromCodePoint;",
            "String.fromCharCode('65');",
            "'x'.toUpperCase(1);",
            "'x'.toLowerCase(1);",
            "const f='x'.toUpperCase;",
            "const f='x'.toLowerCase;",
            "'x'.concat('a','b');",
            "'x'.split();",
            "'x'.slice('1');",
            "'x'.repeat();",
            "'x'.padEnd(2,3);",
            "'x'.replace('x',()=> 'y');",
        ] {
            let source = format!("export function main():void{{{body}}}");
            let error = compile(source.as_bytes(), Limits::default()).unwrap_err();
            assert_eq!(error.code, "C5", "{body}");
        }
        let source = "export function main():void{const constructor=String;}";
        assert_eq!(
            compile(source.as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            "C11"
        );
    }

    #[test]
    fn a_single_member_emits_only_its_dependencies() {
        for (call, expected) in [
            ("concat('y')", vec!["__ulua_str_concat_0"]),
            (
                "slice(0)",
                vec![
                    "__ulua_si",
                    "__ulua_sq",
                    "__ulua_sb",
                    "__ulua_sp",
                    "__ulua_su",
                    "__ulua_ss",
                    "__ulua_str_slice_0",
                ],
            ),
            (
                "indexOf('y')",
                vec![
                    "__ulua_si",
                    "__ulua_sq",
                    "__ulua_sb",
                    "__ulua_sc",
                    "__ulua_sf",
                    "__ulua_str_indexOf_0",
                ],
            ),
            (
                "lastIndexOf('y')",
                vec![
                    "__ulua_si",
                    "__ulua_sq",
                    "__ulua_sb",
                    "__ulua_sc",
                    "__ulua_sf",
                    "__ulua_str_lastIndexOf_0",
                ],
            ),
            (
                "startsWith('y')",
                vec![
                    "__ulua_si",
                    "__ulua_sq",
                    "__ulua_sb",
                    "__ulua_sc",
                    "__ulua_se",
                    "__ulua_str_startsWith_0",
                ],
            ),
            (
                "trim()",
                vec![
                    "__ulua_sn",
                    "__ulua_su",
                    "__ulua_sw",
                    "__ulua_st",
                    "__ulua_str_trim_0",
                ],
            ),
            (
                "trimStart()",
                vec![
                    "__ulua_sn",
                    "__ulua_su",
                    "__ulua_sw",
                    "__ulua_st",
                    "__ulua_str_trimStart_0",
                ],
            ),
            (
                "trimEnd()",
                vec![
                    "__ulua_sn",
                    "__ulua_su",
                    "__ulua_sw",
                    "__ulua_st",
                    "__ulua_str_trimEnd_0",
                ],
            ),
        ] {
            let source = format!("export function main():void{{'x'.{call};}}");
            let luau = compile(source.as_bytes(), Limits::default()).unwrap().luau;
            let actual: Vec<_> = luau
                .lines()
                .filter_map(|line| {
                    let name = line.strip_prefix("local function ")?.split('(').next()?;
                    (name.starts_with("__ulua_s") && !name.starts_with("__ulua_set<"))
                        .then_some(name)
                })
                .collect();
            assert_eq!(actual, expected, "{call}");
        }
    }
}
