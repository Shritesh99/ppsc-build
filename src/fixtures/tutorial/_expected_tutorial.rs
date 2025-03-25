// This file is @generated by ppsc-build.
extern crate alloc;
use parity_scale_codec::{Encode, Decode};

#[derive(Encode, Decode)]
pub struct Person {
    pub name: String,
    pub id: i32,
    pub email: String,
    pub phones: alloc::vec::Vec<person::PhoneNumber>,
}
/// Nested message and enum types in `Person`.
pub mod person {
    extern crate alloc;
    use parity_scale_codec::{Decode, Encode};

    #[derive(Encode, Decode)]
    pub struct PhoneNumber {
        pub number: String,
        pub r#type: i32,
    }
    #[derive(Encode, Decode)]
    pub enum PhoneType {
        Mobile = 0,
        Home = 1,
        Work = 2,
    }
    impl PhoneType {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::Mobile => "MOBILE",
                Self::Home => "HOME",
                Self::Work => "WORK",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> Option<Self> {
            match value {
                "MOBILE" => Some(Self::Mobile),
                "HOME" => Some(Self::Home),
                "WORK" => Some(Self::Work),
                _ => None,
            }
        }
    }
}
