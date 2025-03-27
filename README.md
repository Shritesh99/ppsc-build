# Protocol Buffers Parity SCALE Codec Generator

It generates Rust code from source .proto files using the proto2 or proto3 syntax. It's goal is to make the generated code as simple as possible in [Parity SCALE Codec](https://github.com/paritytech/parity-scale-codec) fromat.

ppsc-build compiles .proto files into Rust.

ppsc-build is designed to be used for build-time code generation as part of a Cargo build-script.

## Usage

Let's create a small library crate, `snazzy`, that defines a collection of snazzy new items in a protobuf file.

```bash
$ cargo new --lib snazzy && cd snazzy
```

First, add `pppsc-build` and `parity-scale-codec` as dependencies to `Cargo.toml`:

```bash
$ cargo add --build ppsc-build
$ cargo add parity-scale-codec
```

### Example

Create a file `src/items.proto` to the project:

```proto
syntax = "proto3";

package snazzy.items;

// A snazzy new shirt!
message Shirt {
    // Label sizes
    enum Size {
        SMALL = 0;
        MEDIUM = 1;
        LARGE = 2;
    }

    // The base color
    string color = 1;
    // The size as stated on the label
    Size size = 2;
}
```

To generate Rust code from `items.proto`, we use `ppsc-build` in the crate's `build.rs` build-script:

```rust
extern create ppsc_build;

fn main() -> Result<()> {
    ppsc_build::compile_protos(&["src/items.proto"], &["src/"])?;
    Ok(())
}
```

Or using the `Config`

```rust
use ppsc_build::Config as Config;

fn main() {
    Config::new()
        .out_dir("src")
        .compile_protos(&["src/items.proto"], &["src/"])
        .unwrap();
}
```

And finally, in `lib.rs`, include the generated code:

```rust
// Include the `items` module, which is generated from items.proto.
// It is important to maintain the same structure as in the proto.
pub mod snazzy {
    pub mod items {
        include!("snazzy.items.rs");
    }
}

use snazzy::items;

/// Returns a large shirt of the specified color
pub fn create_large_shirt(color: String) -> items::Shirt {
    let shirt = items::Shirt {
        color,
        size: items::shirt::Size::Large as i32,
    };
    shirt
}
```

Encoding:

```rust
let shirt = items::Shirt {
    color: "red".to_string(),
    size: items::shirt::Size::Large as i32,
};

let encoded = shirt.encode();
```

Decoding:

```rust
let decoded = items::Shirt::decode(&mut &encoded[..]).unwrap();
```

### Inspirition

-    [Prost](https://github.com/tokio-rs/prost)

### LICENSE

```
MIT
```
