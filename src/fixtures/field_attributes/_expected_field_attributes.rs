// This file is @generated by ppsc-build.
extern crate alloc;
use parity_scale_codec::{Encode, Decode};

#[derive(Encode, Decode)]
pub struct Container {
    pub data: Option<container::Data>,
}
/// Nested message and enum types in `Container`.
pub mod container {
    use super::*;

        #[derive(Encode, Decode)]
    pub enum Data {
        Foo(alloc::boxed::Box<super::Foo>),
        Bar(super::Bar),
    }
}
#[derive(Encode, Decode)]
pub struct Foo {
    pub foo: alloc::string::String,
}
#[derive(Encode, Decode)]
pub struct Bar {
    pub qux: Option<alloc::boxed::Box<Qux>>,
}
#[derive(Encode, Decode)]
pub struct Qux {
}
