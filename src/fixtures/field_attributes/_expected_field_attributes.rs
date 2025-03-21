// This file is @generated by ppsc-build.
#![no_std]
extern crate alloc;
use alloc;
use parity_scale_codec::{Encode, Decode};

#[derive(Encode, Decode)]
pub struct Container {
    pub data: Option<container::Data>,
}
/// Nested message and enum types in `Container`.
pub mod container {
    use parity_scale_codec::{Decode, Encode};

    #[derive(Encode, Decode)]
    pub enum Data {
        Foo(alloc::boxed::Box<super::Foo>),
        Bar(super::Bar),
    }
}
#[derive(Encode, Decode)]
pub struct Foo {
    pub foo: String,
}
#[derive(Encode, Decode)]
pub struct Bar {
    pub qux: Option<alloc::boxed::Box<Qux>>,
}
#[derive(Encode, Decode)]
pub struct Qux {
}
