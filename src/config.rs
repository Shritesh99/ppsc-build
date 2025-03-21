use std::collections::HashMap;
use std::default;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};

use log::trace;

use prost_types::{FileDescriptorProto, FileDescriptorSet};

use crate::BytesType;
use crate::MapType;
use crate::Module;
use crate::ServiceGenerator;
use crate::code_generator::CodeGenerator;
use crate::context::Context;
use crate::extern_paths::ExternPaths;
use crate::message_graph::MessageGraph;
use crate::path::PathMap;

/// Configuration options for Protobuf code generation.
///
/// This configuration builder can be used to set non-default code generation options.
pub struct Config {
    pub(crate) service_generator: Option<Box<dyn ServiceGenerator>>,
    pub(crate) map_type: PathMap<MapType>,
    pub(crate) bytes_type: PathMap<BytesType>,
    pub(crate) type_attributes: PathMap<String>,
    pub(crate) message_attributes: PathMap<String>,
    pub(crate) enum_attributes: PathMap<String>,
    pub(crate) field_attributes: PathMap<String>,
    pub(crate) boxed: PathMap<()>,
    pub(crate) strip_enum_prefix: bool,
    pub(crate) out_dir: Option<PathBuf>,
    pub(crate) extern_paths: Vec<(String, String)>,
    pub(crate) default_package_filename: String,
    pub(crate) type_name_domains: PathMap<String>,
    pub(crate) disable_comments: PathMap<()>,
    pub(crate) skip_debug: PathMap<()>,
    pub(crate) include_file: Option<PathBuf>,
    #[cfg(feature = "format")]
    pub(crate) fmt: bool,
}

impl Config {
    /// Creates a new code generator configuration with default options.
    pub fn new() -> Config {
        Config::default()
    }

    /// Configure the code generator to generate Rust [`BTreeMap`][1] fields for Protobuf
    /// [`map`][2] type fields.
    ///
    /// # Arguments
    ///
    /// **`paths`** - paths to specific fields, messages, or packages which should use a Rust
    /// `BTreeMap` for Protobuf `map` fields. Paths are specified in terms of the Protobuf type
    /// name (not the generated Rust type name). Paths with a leading `.` are treated as fully
    /// qualified names. Paths without a leading `.` are treated as relative, and are suffix
    /// matched on the fully qualified field name. If a Protobuf map field matches any of the
    /// paths, a Rust `BTreeMap` field is generated instead of the default [`HashMap`][3].
    ///
    /// The matching is done on the Protobuf names, before converting to Rust-friendly casing
    /// standards.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = prost_build::Config::new();
    /// // Match a specific field in a message type.
    /// config.btree_map(&[".my_messages.MyMessageType.my_map_field"]);
    ///
    /// // Match all map fields in a message type.
    /// config.btree_map(&[".my_messages.MyMessageType"]);
    ///
    /// // Match all map fields in a package.
    /// config.btree_map(&[".my_messages"]);
    ///
    /// // Match all map fields. Specially useful in `no_std` contexts.
    /// config.btree_map(&["."]);
    ///
    /// // Match all map fields in a nested message.
    /// config.btree_map(&[".my_messages.MyMessageType.MyNestedMessageType"]);
    ///
    /// // Match all fields named 'my_map_field'.
    /// config.btree_map(&["my_map_field"]);
    ///
    /// // Match all fields named 'my_map_field' in messages named 'MyMessageType', regardless of
    /// // package or nesting.
    /// config.btree_map(&["MyMessageType.my_map_field"]);
    ///
    /// // Match all fields named 'my_map_field', and all fields in the 'foo.bar' package.
    /// config.btree_map(&["my_map_field", ".foo.bar"]);
    /// ```
    ///
    /// [1]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
    /// [2]: https://developers.google.com/protocol-buffers/docs/proto3#maps
    /// [3]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
    pub fn btree_map<I, S>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.map_type.clear();
        for matcher in paths {
            self.map_type
                .insert(matcher.as_ref().to_string(), MapType::BTreeMap);
        }
        self
    }

    /// Configure the code generator to generate Rust [`bytes::Bytes`](prost::bytes::Bytes) fields for Protobuf
    /// [`bytes`][2] type fields.
    ///
    /// # Arguments
    ///
    /// **`paths`** - paths to specific fields, messages, or packages which should use a Rust
    /// `Bytes` for Protobuf `bytes` fields. Paths are specified in terms of the Protobuf type
    /// name (not the generated Rust type name). Paths with a leading `.` are treated as fully
    /// qualified names. Paths without a leading `.` are treated as relative, and are suffix
    /// matched on the fully qualified field name. If a Protobuf map field matches any of the
    /// paths, a Rust `Bytes` field is generated instead of the default [`Vec<u8>`][3].
    ///
    /// The matching is done on the Protobuf names, before converting to Rust-friendly casing
    /// standards.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = prost_build::Config::new();
    /// // Match a specific field in a message type.
    /// config.bytes(&[".my_messages.MyMessageType.my_bytes_field"]);
    ///
    /// // Match all bytes fields in a message type.
    /// config.bytes(&[".my_messages.MyMessageType"]);
    ///
    /// // Match all bytes fields in a package.
    /// config.bytes(&[".my_messages"]);
    ///
    /// // Match all bytes fields. Specially useful in `no_std` contexts.
    /// config.bytes(&["."]);
    ///
    /// // Match all bytes fields in a nested message.
    /// config.bytes(&[".my_messages.MyMessageType.MyNestedMessageType"]);
    ///
    /// // Match all fields named 'my_bytes_field'.
    /// config.bytes(&["my_bytes_field"]);
    ///
    /// // Match all fields named 'my_bytes_field' in messages named 'MyMessageType', regardless of
    /// // package or nesting.
    /// config.bytes(&["MyMessageType.my_bytes_field"]);
    ///
    /// // Match all fields named 'my_bytes_field', and all fields in the 'foo.bar' package.
    /// config.bytes(&["my_bytes_field", ".foo.bar"]);
    /// ```
    ///
    /// [2]: https://developers.google.com/protocol-buffers/docs/proto3#scalar
    /// [3]: https://doc.rust-lang.org/std/vec/struct.Vec.html
    pub fn bytes<I, S>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.bytes_type.clear();
        for matcher in paths {
            self.bytes_type
                .insert(matcher.as_ref().to_string(), BytesType::Bytes);
        }
        self
    }

    /// Add additional attribute to matched fields.
    ///
    /// # Arguments
    ///
    /// **`path`** - a path matching any number of fields. These fields get the attribute.
    /// For details about matching fields see [`btree_map`](Self::btree_map).
    ///
    /// **`attribute`** - an arbitrary string that'll be placed before each matched field. The
    /// expected usage are additional attributes, usually in concert with whole-type
    /// attributes set with [`type_attribute`](Self::type_attribute), but it is not
    /// checked and anything can be put there.
    ///
    /// Note that the calls to this method are cumulative ‒ if multiple paths from multiple calls
    /// match the same field, the field gets all the corresponding attributes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = prost_build::Config::new();
    /// // Prost renames fields named `in` to `in_`. But if serialized through serde,
    /// // they should as `in`.
    /// config.field_attribute("in", "#[serde(rename = \"in\")]");
    /// ```
    pub fn field_attribute<P, A>(&mut self, path: P, attribute: A) -> &mut Self
    where
        P: AsRef<str>,
        A: AsRef<str>,
    {
        self.field_attributes
            .insert(path.as_ref().to_string(), attribute.as_ref().to_string());
        self
    }

    /// Add additional attribute to matched messages, enums and one-ofs.
    ///
    /// # Arguments
    ///
    /// **`paths`** - a path matching any number of types. It works the same way as in
    /// [`btree_map`](Self::btree_map), just with the field name omitted.
    ///
    /// **`attribute`** - an arbitrary string to be placed before each matched type. The
    /// expected usage are additional attributes, but anything is allowed.
    ///
    /// The calls to this method are cumulative. They don't overwrite previous calls and if a
    /// type is matched by multiple calls of the method, all relevant attributes are added to
    /// it.
    ///
    /// For things like serde it might be needed to combine with [field
    /// attributes](Self::field_attribute).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = prost_build::Config::new();
    /// // Nothing around uses floats, so we can derive real `Eq` in addition to `PartialEq`.
    /// config.type_attribute(".", "#[derive(Eq)]");
    /// // Some messages want to be serializable with serde as well.
    /// config.type_attribute("my_messages.MyMessageType",
    ///                       "#[derive(Serialize)] #[serde(rename_all = \"snake_case\")]");
    /// config.type_attribute("my_messages.MyMessageType.MyNestedMessageType",
    ///                       "#[derive(Serialize)] #[serde(rename_all = \"snake_case\")]");
    /// ```
    ///
    /// # Oneof fields
    ///
    /// The `oneof` fields don't have a type name of their own inside Protobuf. Therefore, the
    /// field name can be used both with `type_attribute` and `field_attribute` ‒ the first is
    /// placed before the `enum` type definition, the other before the field inside corresponding
    /// message `struct`.
    ///
    /// In other words, to place an attribute on the `enum` implementing the `oneof`, the match
    /// would look like `my_messages.MyMessageType.oneofname`.
    pub fn type_attribute<P, A>(&mut self, path: P, attribute: A) -> &mut Self
    where
        P: AsRef<str>,
        A: AsRef<str>,
    {
        self.type_attributes
            .insert(path.as_ref().to_string(), attribute.as_ref().to_string());
        self
    }

    /// Add additional attribute to matched messages.
    ///
    /// # Arguments
    ///
    /// **`paths`** - a path matching any number of types. It works the same way as in
    /// [`btree_map`](Self::btree_map), just with the field name omitted.
    ///
    /// **`attribute`** - an arbitrary string to be placed before each matched type. The
    /// expected usage are additional attributes, but anything is allowed.
    ///
    /// The calls to this method are cumulative. They don't overwrite previous calls and if a
    /// type is matched by multiple calls of the method, all relevant attributes are added to
    /// it.
    ///
    /// For things like serde it might be needed to combine with [field
    /// attributes](Self::field_attribute).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = prost_build::Config::new();
    /// // Nothing around uses floats, so we can derive real `Eq` in addition to `PartialEq`.
    /// config.message_attribute(".", "#[derive(Eq)]");
    /// // Some messages want to be serializable with serde as well.
    /// config.message_attribute("my_messages.MyMessageType",
    ///                       "#[derive(Serialize)] #[serde(rename_all = \"snake_case\")]");
    /// config.message_attribute("my_messages.MyMessageType.MyNestedMessageType",
    ///                       "#[derive(Serialize)] #[serde(rename_all = \"snake_case\")]");
    /// ```
    pub fn message_attribute<P, A>(&mut self, path: P, attribute: A) -> &mut Self
    where
        P: AsRef<str>,
        A: AsRef<str>,
    {
        self.message_attributes
            .insert(path.as_ref().to_string(), attribute.as_ref().to_string());
        self
    }

    /// Add additional attribute to matched enums and one-ofs.
    ///
    /// # Arguments
    ///
    /// **`paths`** - a path matching any number of types. It works the same way as in
    /// [`btree_map`](Self::btree_map), just with the field name omitted.
    ///
    /// **`attribute`** - an arbitrary string to be placed before each matched type. The
    /// expected usage are additional attributes, but anything is allowed.
    ///
    /// The calls to this method are cumulative. They don't overwrite previous calls and if a
    /// type is matched by multiple calls of the method, all relevant attributes are added to
    /// it.
    ///
    /// For things like serde it might be needed to combine with [field
    /// attributes](Self::field_attribute).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = prost_build::Config::new();
    /// // Nothing around uses floats, so we can derive real `Eq` in addition to `PartialEq`.
    /// config.enum_attribute(".", "#[derive(Eq)]");
    /// // Some messages want to be serializable with serde as well.
    /// config.enum_attribute("my_messages.MyEnumType",
    ///                       "#[derive(Serialize)] #[serde(rename_all = \"snake_case\")]");
    /// config.enum_attribute("my_messages.MyMessageType.MyNestedEnumType",
    ///                       "#[derive(Serialize)] #[serde(rename_all = \"snake_case\")]");
    /// ```
    ///
    /// # Oneof fields
    ///
    /// The `oneof` fields don't have a type name of their own inside Protobuf. Therefore, the
    /// field name can be used both with `enum_attribute` and `field_attribute` ‒ the first is
    /// placed before the `enum` type definition, the other before the field inside corresponding
    /// message `struct`.
    ///
    /// In other words, to place an attribute on the `enum` implementing the `oneof`, the match
    /// would look like `my_messages.MyNestedMessageType.oneofname`.
    pub fn enum_attribute<P, A>(&mut self, path: P, attribute: A) -> &mut Self
    where
        P: AsRef<str>,
        A: AsRef<str>,
    {
        self.enum_attributes
            .insert(path.as_ref().to_string(), attribute.as_ref().to_string());
        self
    }

    /// Wrap matched fields in a `Box`.
    ///
    /// # Arguments
    ///
    /// **`path`** - a path matching any number of fields. These fields get the attribute.
    /// For details about matching fields see [`btree_map`](Self::btree_map).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = prost_build::Config::new();
    /// config.boxed(".my_messages.MyMessageType.my_field");
    /// ```
    pub fn boxed<P>(&mut self, path: P) -> &mut Self
    where
        P: AsRef<str>,
    {
        self.boxed.insert(path.as_ref().to_string(), ());
        self
    }

    /// Configures the code generator to use the provided service generator.
    pub fn service_generator(&mut self, service_generator: Box<dyn ServiceGenerator>) -> &mut Self {
        self.service_generator = Some(service_generator);
        self
    }

    /// Configures the code generator to omit documentation comments on generated Protobuf types.
    ///
    /// # Example
    ///
    /// Occasionally `.proto` files contain code blocks which are not valid Rust. To avoid doctest
    /// failures, annotate the invalid code blocks with an [`ignore` or `no_run` attribute][1], or
    /// disable doctests for the crate with a [Cargo.toml entry][2]. If neither of these options
    /// are possible, then omit comments on generated code during doctest builds:
    ///
    /// ```rust,no_run
    /// # fn main() -> std::io::Result<()> {
    /// let mut config = prost_build::Config::new();
    /// config.disable_comments(&["."]);
    /// config.compile_protos(&["src/frontend.proto", "src/backend.proto"], &["src"])?;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// As with other options which take a set of paths, comments can be disabled on a per-package
    /// or per-symbol basis.
    ///
    /// [1]: https://doc.rust-lang.org/rustdoc/documentation-tests.html#attributes
    /// [2]: https://doc.rust-lang.org/cargo/reference/cargo-targets.html#configuring-a-target
    pub fn disable_comments<I, S>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.disable_comments.clear();
        for matcher in paths {
            self.disable_comments
                .insert(matcher.as_ref().to_string(), ());
        }
        self
    }

    /// Skips generating `impl Debug` for types
    pub fn skip_debug<I, S>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.skip_debug.clear();
        for matcher in paths {
            self.skip_debug.insert(matcher.as_ref().to_string(), ());
        }
        self
    }

    /// Declare an externally provided Protobuf package or type.
    ///
    /// `extern_path` allows `prost` types in external crates to be referenced in generated code.
    ///
    /// When `prost` compiles a `.proto` which includes an import of another `.proto`, it will
    /// automatically recursively compile the imported file as well. `extern_path` can be used
    /// to instead substitute types from an external crate.
    ///
    /// # Example
    ///
    /// As an example, consider a crate, `uuid`, with a `prost`-generated `Uuid` type:
    ///
    /// ```proto
    /// // uuid.proto
    ///
    /// syntax = "proto3";
    /// package uuid;
    ///
    /// message Uuid {
    ///     string uuid_str = 1;
    /// }
    /// ```
    ///
    /// The `uuid` crate implements some traits for `Uuid`, and publicly exports it:
    ///
    /// ```rust,ignore
    /// // lib.rs in the uuid crate
    ///
    /// include!(concat!(env!("OUT_DIR"), "/uuid.rs"));
    ///
    /// pub trait DoSomething {
    ///     fn do_it(&self);
    /// }
    ///
    /// impl DoSomething for Uuid {
    ///     fn do_it(&self) {
    ///         println!("Done");
    ///     }
    /// }
    /// ```
    ///
    /// A separate crate, `my_application`, uses `prost` to generate message types which reference
    /// `Uuid`:
    ///
    /// ```proto
    /// // my_application.proto
    ///
    /// syntax = "proto3";
    /// package my_application;
    ///
    /// import "uuid.proto";
    ///
    /// message MyMessage {
    ///     uuid.Uuid message_id = 1;
    ///     string some_payload = 2;
    /// }
    /// ```
    ///
    /// Additionally, `my_application` depends on the trait impls provided by the `uuid` crate:
    ///
    /// ```rust,ignore
    /// // `main.rs` of `my_application`
    ///
    /// use uuid::{DoSomething, Uuid};
    ///
    /// include!(concat!(env!("OUT_DIR"), "/my_application.rs"));
    ///
    /// pub fn process_message(msg: MyMessage) {
    ///     if let Some(uuid) = msg.message_id {
    ///         uuid.do_it();
    ///     }
    /// }
    /// ```
    ///
    /// Without configuring `uuid` as an external path in `my_application`'s `build.rs`, `prost`
    /// would compile a completely separate version of the `Uuid` type, and `process_message` would
    /// fail to compile. However, if `my_application` configures `uuid` as an extern path with a
    /// call to `.extern_path(".uuid", "::uuid")`, `prost` will use the external type instead of
    /// compiling a new version of `Uuid`. Note that the configuration could also be specified as
    /// `.extern_path(".uuid.Uuid", "::uuid::Uuid")` if only the `Uuid` type were externally
    /// provided, and not the whole `uuid` package.
    ///
    /// # Usage
    ///
    /// `extern_path` takes a fully-qualified Protobuf path, and the corresponding Rust path that
    /// it will be substituted with in generated code. The Protobuf path can refer to a package or
    /// a type, and the Rust path should correspondingly refer to a Rust module or type.
    ///
    /// ```rust
    /// # let mut config = prost_build::Config::new();
    /// // Declare the `uuid` Protobuf package and all nested packages and types as externally
    /// // provided by the `uuid` crate.
    /// config.extern_path(".uuid", "::uuid");
    ///
    /// // Declare the `foo.bar.baz` Protobuf package and all nested packages and types as
    /// // externally provided by the `foo_bar_baz` crate.
    /// config.extern_path(".foo.bar.baz", "::foo_bar_baz");
    ///
    /// // Declare the `uuid.Uuid` Protobuf type (and all nested types) as externally provided
    /// // by the `uuid` crate's `Uuid` type.
    /// config.extern_path(".uuid.Uuid", "::uuid::Uuid");
    /// ```
    pub fn extern_path<P1, P2>(&mut self, proto_path: P1, rust_path: P2) -> &mut Self
    where
        P1: Into<String>,
        P2: Into<String>,
    {
        self.extern_paths
            .push((proto_path.into(), rust_path.into()));
        self
    }

    /// Configures the code generator to not strip the enum name from variant names.
    ///
    /// Protobuf enum definitions commonly include the enum name as a prefix of every variant name.
    /// This style is non-idiomatic in Rust, so by default `prost` strips the enum name prefix from
    /// variants which include it. Configuring this option prevents `prost` from stripping the
    /// prefix.
    pub fn retain_enum_prefix(&mut self) -> &mut Self {
        self.strip_enum_prefix = false;
        self
    }

    /// Configures the output directory where generated Rust files will be written.
    ///
    /// If unset, defaults to the `OUT_DIR` environment variable. `OUT_DIR` is set by Cargo when
    /// executing build scripts, so `out_dir` typically does not need to be configured.
    pub fn out_dir<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>,
    {
        self.out_dir = Some(path.into());
        self
    }

    /// Configures what filename protobufs with no package definition are written to.
    /// The filename will be appended with the `.rs` extension.
    pub fn default_package_filename<S>(&mut self, filename: S) -> &mut Self
    where
        S: Into<String>,
    {
        self.default_package_filename = filename.into();
        self
    }

    /// Specify domain names to use with message type URLs.
    ///
    /// # Domains
    ///
    /// **`paths`** - a path matching any number of types. It works the same way as in
    /// [`btree_map`](Self::btree_map), just with the field name omitted.
    ///
    /// **`domain`** - an arbitrary string to be used as a prefix for type URLs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # let mut config = prost_build::Config::new();
    /// // Full type URL of the message `google.profile.Person`,
    /// // will be `type.googleapis.com/google.profile.Person`.
    /// config.type_name_domain(&["."], "type.googleapis.com");
    /// ```
    pub fn type_name_domain<I, S, D>(&mut self, paths: I, domain: D) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
        D: AsRef<str>,
    {
        for matcher in paths {
            self.type_name_domains
                .insert(matcher.as_ref().to_string(), domain.as_ref().to_string());
        }
        self
    }

    /// Configures the optional module filename for easy inclusion of all generated Rust files
    ///
    /// If set, generates a file (inside the `OUT_DIR` or `out_dir()` as appropriate) which contains
    /// a set of `pub mod XXX` statements combining to load all Rust files generated.  This can allow
    /// for a shortcut where multiple related proto files have been compiled together resulting in
    /// a semi-complex set of includes.
    ///
    /// Turning a need for:
    ///
    /// ```rust,no_run,ignore
    /// pub mod Foo {
    ///     pub mod Bar {
    ///         include!(concat!(env!("OUT_DIR"), "/foo.bar.rs"));
    ///     }
    ///     pub mod Baz {
    ///         include!(concat!(env!("OUT_DIR"), "/foo.baz.rs"));
    ///     }
    /// }
    /// ```
    ///
    /// Into the simpler:
    ///
    /// ```rust,no_run,ignore
    /// include!(concat!(env!("OUT_DIR"), "/_includes.rs"));
    /// ```
    pub fn include_file<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>,
    {
        self.include_file = Some(path.into());
        self
    }

    // IMPROVEMENT: https://github.com/tokio-rs/prost/pull/1022/files#r1563818651
    /// Configures the code generator to format the output code via `prettyplease`.
    ///
    /// By default, this is enabled but if the `format` feature is not enabled this does
    /// nothing.
    #[cfg(feature = "format")]
    pub fn format(&mut self, enabled: bool) -> &mut Self {
        self.fmt = enabled;
        self
    }

    /// Compile a [`FileDescriptorSet`] into Rust files during a Cargo build with
    /// additional code generator configuration options.
    ///
    /// This method is like `compile_protos` function except it does not invoke `protoc`
    /// and instead requires the user to supply a [`FileDescriptorSet`].
    ///
    /// # Example `build.rs`
    ///
    /// ```rust,no_run
    /// # use prost_types::FileDescriptorSet;
    /// # fn fds() -> FileDescriptorSet { todo!() }
    /// fn main() -> std::io::Result<()> {
    ///   let file_descriptor_set = fds();
    ///
    ///   prost_build::Config::new()
    ///     .compile_fds(file_descriptor_set)
    /// }
    /// ```
    pub fn compile_fds(&mut self, fds: FileDescriptorSet) -> Result<()> {
        let mut target_is_env = false;
        let target: PathBuf = self.out_dir.clone().map(Ok).unwrap_or_else(|| {
            env::var_os("OUT_DIR")
                .ok_or_else(|| {
                    Error::new(ErrorKind::Other, "OUT_DIR environment variable is not set")
                })
                .map(|val| {
                    target_is_env = true;
                    Into::into(val)
                })
        })?;

        let requests = fds
            .file
            .into_iter()
            .map(|descriptor| {
                (
                    Module::from_protobuf_package_name(descriptor.package()),
                    descriptor,
                )
            })
            .collect::<Vec<_>>();

        let file_names = requests
            .iter()
            .map(|req| {
                (
                    req.0.clone(),
                    req.0.to_file_name_or(&self.default_package_filename),
                )
            })
            .collect::<HashMap<Module, String>>();

        let modules = self.generate(requests)?;
        for (module, content) in &modules {
            let file_name = file_names
                .get(module)
                .expect("every module should have a filename");
            let output_path = target.join(file_name);

            write_file_if_changed(&output_path, content.as_bytes())?;
        }

        if let Some(ref include_file) = self.include_file {
            let path = target.join(include_file);
            trace!("Writing include file: {}", path.display());
            let mut buffer = Vec::new();
            self.write_line(&mut buffer, 0, "// This file is @generated by ppsc-build.")?;
            self.write_includes(
                modules.keys().collect(),
                &mut buffer,
                if target_is_env { None } else { Some(&target) },
                &file_names,
            )?;

            write_file_if_changed(&path, &buffer)?;
        }

        Ok(())
    }

    /// Compile `.proto` files into Rust files during a Cargo build with additional code generator
    /// configuration options.
    ///
    /// This method is like the `prost_build::compile_protos` function, with the added ability to
    /// specify non-default code generation options. See that function for more information about
    /// the arguments and generated outputs.
    ///
    /// The `protos` and `includes` arguments are ignored if `skip_protoc_run` is specified.
    ///
    /// # Example `build.rs`
    ///
    /// ```rust,no_run
    /// # use std::io::Result;
    /// fn main() -> Result<()> {
    ///   let mut prost_build = prost_build::Config::new();
    ///   prost_build.btree_map(&["."]);
    ///   prost_build.compile_protos(&["src/frontend.proto", "src/backend.proto"], &["src"])?;
    ///   Ok(())
    /// }
    /// ```
    pub fn compile_protos(
        &mut self,
        protos: &[impl AsRef<Path>],
        includes: &[impl AsRef<Path>],
    ) -> Result<()> {
        let file_descriptor_set = protox::compile(protos, includes).map_err(|error| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Failed to compile FileDiscriptorSet: {}", error),
            )
        })?;

        self.compile_fds(file_descriptor_set)
    }

    pub(crate) fn write_includes(
        &self,
        mut modules: Vec<&Module>,
        outfile: &mut impl Write,
        basepath: Option<&PathBuf>,
        file_names: &HashMap<Module, String>,
    ) -> Result<()> {
        modules.sort();

        let mut stack = Vec::new();

        for module in modules {
            while !module.starts_with(&stack) {
                stack.pop();
                self.write_line(outfile, stack.len(), "}")?;
            }
            while stack.len() < module.len() {
                self.write_line(
                    outfile,
                    stack.len(),
                    &format!("pub mod {} {{", module.part(stack.len())),
                )?;
                stack.push(module.part(stack.len()).to_owned());
            }

            let file_name = file_names
                .get(module)
                .expect("every module should have a filename");

            if basepath.is_some() {
                self.write_line(
                    outfile,
                    stack.len(),
                    &format!("include!(\"{}\");", file_name),
                )?;
            } else {
                self.write_line(
                    outfile,
                    stack.len(),
                    &format!("include!(concat!(env!(\"OUT_DIR\"), \"/{}\"));", file_name),
                )?;
            }
        }

        for depth in (0..stack.len()).rev() {
            self.write_line(outfile, depth, "}")?;
        }

        Ok(())
    }

    fn write_line(&self, outfile: &mut impl Write, depth: usize, line: &str) -> Result<()> {
        outfile.write_all(format!("{}{}\n", ("    ").to_owned().repeat(depth), line).as_bytes())
    }

    /// Processes a set of modules and file descriptors, returning a map of modules to generated
    /// code contents.
    ///
    /// This is generally used when control over the output should not be managed by Prost,
    /// such as in a flow for a `protoc` code generating plugin. When compiling as part of a
    /// `build.rs` file, instead use [`Self::compile_protos()`].
    pub fn generate(
        &mut self,
        requests: Vec<(Module, FileDescriptorProto)>,
    ) -> Result<HashMap<Module, String>> {
        let mut modules = HashMap::new();
        let mut packages = HashMap::new();

        let message_graph = MessageGraph::new(requests.iter().map(|x| &x.1));
        let extern_paths = ExternPaths::new(&self.extern_paths)
            .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
        let mut context = Context::new(self, message_graph, extern_paths);

        for (request_module, request_fd) in requests {
            // Only record packages that have services
            if !request_fd.service.is_empty() {
                packages.insert(request_module.clone(), request_fd.package().to_string());
            }
            let buf = modules
                .entry(request_module.clone())
                .or_insert_with(String::new);
            CodeGenerator::generate(&mut context, request_fd, buf);
            if buf.is_empty() {
                // Did not generate any code, remove from list to avoid inclusion in include file or output file list
                modules.remove(&request_module);
            }
        }

        if let Some(service_generator) = context.service_generator_mut() {
            for (module, package) in packages {
                let buf = modules.get_mut(&module).unwrap();
                service_generator.finalize_package(&package, buf);
            }
        }

        #[cfg(feature = "format")]
        if self.fmt {
            for buf in modules.values_mut() {
                let file = syn::parse_file(buf).unwrap();
                let formatted = prettyplease::unparse(&file);
                *buf = formatted;
            }
        }

        self.add_generated_modules(&mut modules);

        Ok(modules)
    }

    fn add_generated_modules(&mut self, modules: &mut HashMap<Module, String>) {
        for buf in modules.values_mut() {
            let with_generated = "// This file is @generated by ppsc-build.\n".to_string() + buf;
            *buf = with_generated;
        }
    }
}

/// Write a slice as the entire contents of a file.
///
/// This function will create a file if it does not exist,
/// and will entirely replace its contents if it does. When
/// the contents is already correct, it doesn't touch to the file.
fn write_file_if_changed(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let previous_content = fs::read(path);

    if previous_content
        .map(|previous_content| previous_content == content)
        .unwrap_or(false)
    {
        trace!("unchanged: {}", path.display());
        Ok(())
    } else {
        trace!("writing: {}", path.display());
        fs::write(path, content)
    }
}

impl default::Default for Config {
    fn default() -> Config {
        Config {
            service_generator: None,
            map_type: PathMap::default(),
            bytes_type: PathMap::default(),
            type_attributes: PathMap::default(),
            message_attributes: PathMap::default(),
            enum_attributes: PathMap::default(),
            field_attributes: PathMap::default(),
            boxed: PathMap::default(),
            strip_enum_prefix: true,
            out_dir: None,
            extern_paths: Vec::new(),
            default_package_filename: "_".to_string(),
            type_name_domains: PathMap::default(),
            disable_comments: PathMap::default(),
            skip_debug: PathMap::default(),
            include_file: None,
            #[cfg(feature = "format")]
            fmt: true,
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Config")
            .field("service_generator", &self.service_generator.is_some())
            .field("map_type", &self.map_type)
            .field("bytes_type", &self.bytes_type)
            .field("type_attributes", &self.type_attributes)
            .field("field_attributes", &self.field_attributes)
            .field("strip_enum_prefix", &self.strip_enum_prefix)
            .field("out_dir", &self.out_dir)
            .field("extern_paths", &self.extern_paths)
            .field("default_package_filename", &self.default_package_filename)
            .field("type_name_domains", &self.type_name_domains)
            .field("disable_comments", &self.disable_comments)
            .field("skip_debug", &self.skip_debug)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_starts_with {
        ($left:expr, $right:expr) => {
            match (&$left, &$right) {
                (left_val, right_val) => {
                    if !(left_val.starts_with(right_val)) {
                        panic!(
                            "assertion 'starts_with` failed:\nleft: {}\nright: {}",
                            left_val, right_val
                        )
                    }
                }
            }
        };
    }

    #[test]
    fn test_error_unset_out_dir() {
        let mut config = Config::new();

        let err = config
            .compile_fds(FileDescriptorSet::default())
            .unwrap_err();
        assert_eq!(err.to_string(), "OUT_DIR environment variable is not set")
    }
}
