//! Ordered collections have a closed member set and an opaque runtime representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TableMethod {
    Begin,
    End,
    New,
    FromArray,
    Get,
    GetOr,
    Set,
    Add,
    Has,
    Delete,
    Clear,
    Each,
    Keys,
    Values,
}
impl TableMethod {
    /// The helper discriminator is zero for numbers and one for all other admitted keys.
    pub(crate) fn key_kind(key: crate::types::Type) -> usize {
        usize::from(key != crate::types::Type::Number)
    }

    pub(crate) fn keyed(self) -> bool {
        !matches!(
            self,
            Self::Begin | Self::End | Self::Clear | Self::Each | Self::Keys | Self::Values
        )
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Begin => "table_begin",
            Self::End => "table_end",
            Self::New => "table_new",
            Self::FromArray => "table_fromArray",
            Self::Get => "table_get",
            Self::GetOr => "table_getOr",
            Self::Set => "table_set",
            Self::Add => "table_add",
            Self::Has => "table_has",
            Self::Delete => "table_delete",
            Self::Clear => "table_clear",
            Self::Each => "table_each",
            Self::Keys => "table_keys",
            Self::Values => "table_values",
        }
    }
}
