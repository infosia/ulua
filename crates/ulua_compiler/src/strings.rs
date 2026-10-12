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
}
impl StringMethod {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        Some(match name {
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
    fn string_surface_rejects_unlisted_members_and_forms() {
        for body in [
            "const c='x'[0];",
            "const x='a'<'b';",
            "const f='x'.slice;",
            "'x'.toUpperCase();",
            "'x'.toLowerCase();",
            "'x'.toLocaleUpperCase();",
            "'x'.toLocaleLowerCase();",
            "'x'.localeCompare('y');",
            "'x'.normalize();",
            "'x'.match('x');",
            "'x'.matchAll('x');",
            "'x'.search('x');",
            "'x'.replace(/x/,'y');",
            "'x'.replaceAll(/x/g,'y');",
            "String.fromCharCode(65);",
            "String.raw('x');",
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
        let source = "export function main():void{print('x'.concat('y'));}";
        let luau = compile(source.as_bytes(), Limits::default()).unwrap().luau;
        assert!(luau.contains("local function __ulua_str_concat_0"));
        for unused in [
            "__ulua_str_slice_0",
            "__ulua_str_repeat_0",
            "local function __ulua_sb",
            "local function __ulua_si",
        ] {
            assert!(!luau.contains(unused), "{unused}");
        }
    }
}
