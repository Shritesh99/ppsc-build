/// The map collection type to output for Protobuf `map` fields.
#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(crate) enum MapType {
    /// The [`alloc::collections::BTreeMap`] type.
    #[default]
    HashMap,
    /// The [`alloc::collections::BTreeMap`] type.
    BTreeMap,
}

/// The bytes collection type to output for Protobuf `bytes` fields.
#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(crate) enum BytesType {
    /// The [`alloc::vec::Vec<u8>`] type.
    #[default]
    Vec,
    /// The [`bytes::Bytes`](prost::bytes::Bytes) type.
    Bytes,
}

impl MapType {
    /// The fully-qualified Rust type corresponding to the map type.
    pub fn rust_type(&self) -> &'static str {
        match self {
            MapType::HashMap => "alloc::collections::BTreeMap",
            MapType::BTreeMap => "alloc::collections::BTreeMap",
        }
    }
}

impl BytesType {
    /// The fully-qualified Rust type corresponding to the bytes type.
    pub fn rust_type(&self) -> &'static str {
        match self {
            BytesType::Vec => "alloc::vec::Vec<u8>",
            BytesType::Bytes => "Bytes",
        }
    }
}
