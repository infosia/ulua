//! Closed array and string method kinds. Callback arities remain explicit in the checked form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Method {
    Table(crate::tables::TableMethod),
    String(crate::strings::StringMethod),
    IndexOf,
    LastIndexOf,
    Includes,
    Join,
    Slice,
    Concat,
    Fill,
    Reverse,
    Splice,
    Unshift,
    CopyWithin,
    At,
    Pop,
    Shift,
    Clear,
    ForEach,
    Map,
    Filter,
    Some,
    Every,
    Find,
    FindLast,
    FindIndex,
    FindLastIndex,
    FlatMap,
    Reduce,
    ReduceRight,
    Sort,
}

impl Method {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "indexOf" => Self::IndexOf,
            "lastIndexOf" => Self::LastIndexOf,
            "includes" => Self::Includes,
            "join" => Self::Join,
            "slice" => Self::Slice,
            "concat" => Self::Concat,
            "fill" => Self::Fill,
            "reverse" => Self::Reverse,
            "splice" => Self::Splice,
            "unshift" => Self::Unshift,
            "copyWithin" => Self::CopyWithin,
            "at" => Self::At,
            "pop" => Self::Pop,
            "shift" => Self::Shift,
            "clear" => Self::Clear,
            "forEach" => Self::ForEach,
            "map" => Self::Map,
            "filter" => Self::Filter,
            "some" => Self::Some,
            "every" => Self::Every,
            "find" => Self::Find,
            "findLast" => Self::FindLast,
            "findIndex" => Self::FindIndex,
            "findLastIndex" => Self::FindLastIndex,
            "flatMap" => Self::FlatMap,
            "reduce" => Self::Reduce,
            "reduceRight" => Self::ReduceRight,
            "sort" => Self::Sort,
            _ => return None,
        })
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Table(m) => m.name(),
            Self::String(m) => m.name(),
            Self::IndexOf => "indexOf",
            Self::LastIndexOf => "lastIndexOf",
            Self::Includes => "includes",
            Self::Join => "join",
            Self::Slice => "slice",
            Self::Concat => "concat",
            Self::Fill => "fill",
            Self::Reverse => "reverse",
            Self::Splice => "splice",
            Self::Unshift => "unshift",
            Self::CopyWithin => "copyWithin",
            Self::At => "at",
            Self::Pop => "pop",
            Self::Shift => "shift",
            Self::Clear => "clear",
            Self::ForEach => "forEach",
            Self::Map => "map",
            Self::Filter => "filter",
            Self::Some => "some",
            Self::Every => "every",
            Self::Find => "find",
            Self::FindLast => "findLast",
            Self::FindIndex => "findIndex",
            Self::FindLastIndex => "findLastIndex",
            Self::FlatMap => "flatMap",
            Self::Reduce => "reduce",
            Self::ReduceRight => "reduceRight",
            Self::Sort => "sort",
        }
    }

    pub(crate) fn arities(self) -> std::ops::RangeInclusive<usize> {
        match self {
            Self::Table(_) => 0..=0,
            Self::String(_) => 0..=0,
            Self::IndexOf
            | Self::LastIndexOf
            | Self::Includes
            | Self::Join
            | Self::Slice
            | Self::Concat
            | Self::Fill
            | Self::Reverse
            | Self::Splice
            | Self::Unshift
            | Self::CopyWithin
            | Self::At
            | Self::Pop
            | Self::Shift
            | Self::Clear => 0..=0,
            Self::ForEach
            | Self::Map
            | Self::Filter
            | Self::Some
            | Self::Every
            | Self::Find
            | Self::FindLast
            | Self::FindIndex
            | Self::FindLastIndex
            | Self::FlatMap => 1..=2,
            Self::Reduce | Self::ReduceRight => 2..=3,
            Self::Sort => 2..=2,
        }
    }
}
