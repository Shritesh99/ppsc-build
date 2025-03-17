use std::io::Result;
use std::path::Path;

use prost_types::FileDescriptorSet;

mod ast;
pub use crate::ast::{Comments, Method, Service};

mod collections;
pub(crate) use collections::{BytesType, MapType};

mod code_generator;
mod context;
mod extern_paths;
mod ident;
mod message_graph;
mod path;

mod config;
pub use config::Config;

mod module;
pub use module::Module;

pub trait ServiceGenerator {
    /// Generates a Rust interface or implementation for a service, writing the
    /// result to `buf`.
    fn generate(&mut self, service: Service, buf: &mut String);

    /// Finalizes the generation process.
    ///
    /// In case there's something that needs to be output at the end of the generation process, it
    /// goes here. Similar to [`generate`](Self::generate), the output should be appended to
    /// `buf`.
    ///
    /// An example can be a module or other thing that needs to appear just once, not for each
    /// service generated.
    ///
    /// This still can be called multiple times in a lifetime of the service generator, because it
    /// is called once per `.proto` file.
    ///
    /// The default implementation is empty and does nothing.
    fn finalize(&mut self, _buf: &mut String) {}

    /// Finalizes the generation process for an entire protobuf package.
    ///
    /// This differs from [`finalize`](Self::finalize) by where (and how often) it is called
    /// during the service generator life cycle. This method is called once per protobuf package,
    /// making it ideal for grouping services within a single package spread across multiple
    /// `.proto` files.
    ///
    /// The default implementation is empty and does nothing.
    fn finalize_package(&mut self, _package: &str, _buf: &mut String) {}
}

/// Compile `.proto` files into Rust files during a Cargo build.
///
/// The generated `.rs` files are written to the Cargo `OUT_DIR` directory, suitable for use with
/// the [include!][1] macro. See the [Cargo `build.rs` code generation][2] example for more info.
///
/// This function should be called in a project's `build.rs`.
///
/// # Arguments
///
/// **`protos`** - Paths to `.proto` files to compile. Any transitively [imported][3] `.proto`
/// files are automatically be included.
///
/// **`includes`** - Paths to directories in which to search for imports. Directories are searched
/// in order. The `.proto` files passed in **`protos`** must be found in one of the provided
/// include directories.
///
/// # Errors
///
/// This function can fail for a number of reasons:
///
///   - Failure to locate or download `protoc`.
///   - Failure to parse the `.proto`s.
///   - Failure to locate an imported `.proto`.
///   - Failure to compile a `.proto` without a [package specifier][4].
///
/// It's expected that this function call be `unwrap`ed in a `build.rs`; there is typically no
/// reason to gracefully recover from errors during a build.
///
/// # Example `build.rs`
///
/// ```rust,no_run
/// # use std::io::Result;
/// fn main() -> Result<()> {
///   prost_build::compile_protos(&["src/frontend.proto", "src/backend.proto"], &["src"])?;
///   Ok(())
/// }
/// ```
///
/// [1]: https://doc.rust-lang.org/std/macro.include.html
/// [2]: http://doc.crates.io/build-script.html#case-study-code-generation
/// [3]: https://developers.google.com/protocol-buffers/docs/proto3#importing-definitions
/// [4]: https://developers.google.com/protocol-buffers/docs/proto#packages
pub fn compile_protos(protos: &[impl AsRef<Path>], includes: &[impl AsRef<Path>]) -> Result<()> {
    Config::new().compile_protos(protos, includes)
}

/// Compile a [`FileDescriptorSet`] into Rust files during a Cargo build.
///
/// The generated `.rs` files are written to the Cargo `OUT_DIR` directory, suitable for use with
/// the [include!][1] macro. See the [Cargo `build.rs` code generation][2] example for more info.
///
/// This function should be called in a project's `build.rs`.
///
/// This function can be combined with a crate like [`protox`] which outputs a
/// [`FileDescriptorSet`] and is a pure Rust implementation of `protoc`.
///
/// # Example
/// ```rust,no_run
/// # use prost_types::FileDescriptorSet;
/// # fn fds() -> FileDescriptorSet { todo!() }
/// fn main() -> std::io::Result<()> {
///   let file_descriptor_set = fds();
///
///   prost_build::compile_fds(file_descriptor_set)
/// }
/// ```
///
/// [`protox`]: https://github.com/andrewhickman/protox
/// [1]: https://doc.rust-lang.org/std/macro.include.html
/// [2]: http://doc.crates.io/build-script.html#case-study-code-generation
pub fn compile_fds(fds: FileDescriptorSet) -> Result<()> {
    Config::new().compile_fds(fds)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs::File;
    use std::io::Read;
    use std::rc::Rc;

    use super::*;

    macro_rules! assert_eq_fixture_file {
        ($expected_path:expr, $actual_path:expr) => {{
            let actual = std::fs::read_to_string($actual_path).unwrap();

            // Normalizes windows and Linux-style EOL
            let actual = actual.replace("\r\n", "\n");

            assert_eq_fixture_contents!($expected_path, actual);
        }};
    }

    macro_rules! assert_eq_fixture_contents {
        ($expected_path:expr, $actual:expr) => {{
            let expected = std::fs::read_to_string($expected_path).unwrap();

            // Normalizes windows and Linux-style EOL
            let expected = expected.replace("\r\n", "\n");

            if expected != $actual {
                std::fs::write($expected_path, &$actual).unwrap();
            }

            assert_eq!(expected, $actual);
        }};
    }

    /// An example service generator that generates a trait with methods corresponding to the
    /// service methods.
    struct ServiceTraitGenerator;

    impl ServiceGenerator for ServiceTraitGenerator {
        fn generate(&mut self, service: Service, buf: &mut String) {
            // Generate a trait for the service.
            service.comments.append_with_indent(0, buf);
            buf.push_str(&format!("trait {} {{\n", &service.name));

            // Generate the service methods.
            for method in service.methods {
                method.comments.append_with_indent(1, buf);
                buf.push_str(&format!(
                    "    fn {}(_: {}) -> {};\n",
                    method.name, method.input_type, method.output_type
                ));
            }

            // Close out the trait.
            buf.push_str("}\n");
        }
        fn finalize(&mut self, buf: &mut String) {
            // Needs to be present only once, no matter how many services there are
            buf.push_str("pub mod utils { }\n");
        }
    }

    /// Implements `ServiceGenerator` and provides some state for assertions.
    struct MockServiceGenerator {
        state: Rc<RefCell<MockState>>,
    }

    /// Holds state for `MockServiceGenerator`
    #[derive(Default)]
    struct MockState {
        service_names: Vec<String>,
        package_names: Vec<String>,
        finalized: u32,
    }

    impl MockServiceGenerator {
        fn new(state: Rc<RefCell<MockState>>) -> Self {
            Self { state }
        }
    }

    impl ServiceGenerator for MockServiceGenerator {
        fn generate(&mut self, service: Service, _buf: &mut String) {
            let mut state = self.state.borrow_mut();
            state.service_names.push(service.name);
        }

        fn finalize(&mut self, _buf: &mut String) {
            let mut state = self.state.borrow_mut();
            state.finalized += 1;
        }

        fn finalize_package(&mut self, package: &str, _buf: &mut String) {
            let mut state = self.state.borrow_mut();
            state.package_names.push(package.to_string());
        }
    }

    #[test]
    fn smoke_test() {
        let _ = env_logger::try_init();
        let tempdir = tempfile::tempdir().unwrap();

        Config::new()
            .service_generator(Box::new(ServiceTraitGenerator))
            .out_dir(tempdir.path())
            .compile_protos(&["src/fixtures/smoke_test/smoke_test.proto"], &["src"])
            .unwrap();
    }

    #[test]
    fn finalize_package() {
        let _ = env_logger::try_init();
        let tempdir = tempfile::tempdir().unwrap();

        let state = Rc::new(RefCell::new(MockState::default()));
        let generator = MockServiceGenerator::new(Rc::clone(&state));

        Config::new()
            .service_generator(Box::new(generator))
            .include_file("_protos.rs")
            .out_dir(tempdir.path())
            .compile_protos(
                &[
                    "src/fixtures/helloworld/hello.proto",
                    "src/fixtures/helloworld/goodbye.proto",
                ],
                &["src/fixtures/helloworld"],
            )
            .unwrap();

        let state = state.borrow();
        assert_eq!(&state.service_names, &["Greeting", "Farewell"]);
        assert_eq!(&state.package_names, &["helloworld"]);
        assert_eq!(state.finalized, 3);
    }

    #[test]
    fn test_generate_message_attributes() {
        let _ = env_logger::try_init();
        let tempdir = tempfile::tempdir().unwrap();

        let mut config = Config::new();
        config
            .out_dir(tempdir.path())
            // Add attributes to all messages and enums
            .message_attribute(".", "#[derive(derive_builder::Builder)]")
            .enum_attribute(".", "#[some_enum_attr(u8)]");

        let fds = config
            .load_fds(
                &["src/fixtures/helloworld/hello.proto"],
                &["src/fixtures/helloworld"],
            )
            .unwrap();

        // Add custom attributes to messages that are service inputs or outputs.
        for file in &fds.file {
            for service in &file.service {
                for method in &service.method {
                    if let Some(input) = &method.input_type {
                        config.message_attribute(input, "#[derive(custom_proto::Input)]");
                    }
                    if let Some(output) = &method.output_type {
                        config.message_attribute(output, "#[derive(custom_proto::Output)]");
                    }
                }
            }
        }

        config.compile_fds(fds).unwrap();

        assert_eq_fixture_file!(
            if cfg!(feature = "format") {
                "src/fixtures/helloworld/_expected_helloworld_formatted.rs"
            } else {
                "src/fixtures/helloworld/_expected_helloworld.rs"
            },
            tempdir.path().join("helloworld.rs")
        );
    }

    #[test]
    fn test_generate_no_empty_outputs() {
        let _ = env_logger::try_init();
        let state = Rc::new(RefCell::new(MockState::default()));
        let generator = MockServiceGenerator::new(Rc::clone(&state));
        let include_file = "_include.rs";
        let tempdir = tempfile::tempdir().unwrap();
        let previously_empty_proto_path = tempdir.path().join(Path::new("google.protobuf.rs"));

        Config::new()
            .service_generator(Box::new(generator))
            .include_file(include_file)
            .out_dir(tempdir.path())
            .compile_protos(
                &["src/fixtures/imports_empty/imports_empty.proto"],
                &["src/fixtures/imports_empty"],
            )
            .unwrap();

        // Prior to PR introducing this test, the generated include file would have the file
        // google.protobuf.rs which was an empty file. Now that file should only exist if it has content
        if let Ok(mut f) = File::open(previously_empty_proto_path) {
            // Since this file was generated, it should not be empty.
            let mut contents = String::new();
            f.read_to_string(&mut contents).unwrap();
            assert!(!contents.is_empty());
        } else {
            // The file wasn't generated so the result include file should not reference it
            assert_eq_fixture_file!(
                "src/fixtures/imports_empty/_expected_include.rs",
                tempdir.path().join(Path::new(include_file))
            );
        }
    }

    #[test]
    fn test_generate_field_attributes() {
        let _ = env_logger::try_init();
        let tempdir = tempfile::tempdir().unwrap();

        Config::new()
            .out_dir(tempdir.path())
            .boxed("Container.data.foo")
            .boxed("Bar.qux")
            .compile_protos(
                &["src/fixtures/field_attributes/field_attributes.proto"],
                &["src/fixtures/field_attributes"],
            )
            .unwrap();

        assert_eq_fixture_file!(
            if cfg!(feature = "format") {
                "src/fixtures/field_attributes/_expected_field_attributes_formatted.rs"
            } else {
                "src/fixtures/field_attributes/_expected_field_attributes.rs"
            },
            tempdir.path().join("field_attributes.rs")
        );
    }

    #[test]
    fn deterministic_include_file() {
        let _ = env_logger::try_init();

        for _ in 1..10 {
            let state = Rc::new(RefCell::new(MockState::default()));
            let generator = MockServiceGenerator::new(Rc::clone(&state));
            let include_file = "_include.rs";
            let tempdir = tempfile::tempdir().unwrap();

            Config::new()
                .service_generator(Box::new(generator))
                .include_file(include_file)
                .out_dir(tempdir.path())
                .compile_protos(
                    &[
                        "src/fixtures/alphabet/a.proto",
                        "src/fixtures/alphabet/b.proto",
                        "src/fixtures/alphabet/c.proto",
                        "src/fixtures/alphabet/d.proto",
                        "src/fixtures/alphabet/e.proto",
                        "src/fixtures/alphabet/f.proto",
                    ],
                    &["src/fixtures/alphabet"],
                )
                .unwrap();

            assert_eq_fixture_file!(
                "src/fixtures/alphabet/_expected_include.rs",
                tempdir.path().join(Path::new(include_file))
            );
        }
    }

    #[test]
    fn write_includes() {
        let modules = [
            Module::from_protobuf_package_name("foo.bar.baz"),
            Module::from_protobuf_package_name(""),
            Module::from_protobuf_package_name("foo.bar"),
            Module::from_protobuf_package_name("bar"),
            Module::from_protobuf_package_name("foo"),
            Module::from_protobuf_package_name("foo.bar.qux"),
            Module::from_protobuf_package_name("foo.bar.a.b.c"),
        ];

        let file_names = modules
            .iter()
            .map(|m| (m.clone(), m.to_file_name_or("_.default")))
            .collect();

        let mut buf = Vec::new();
        Config::new()
            .default_package_filename("_.default")
            .write_includes(modules.iter().collect(), &mut buf, None, &file_names)
            .unwrap();
        let actual = String::from_utf8(buf).unwrap();
        assert_eq_fixture_contents!("src/fixtures/write_includes/_.includes.rs", actual);
    }
}
