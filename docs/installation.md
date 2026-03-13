# Installation & Building

`query-rs` is written in Rust and can be built from source or installed via the provided script.

## Installation
Run the following command
```bash
curl -fsSL https://raw.githubusercontent.com/felixdayapper/query.rs/master/install.sh | sh
```

## Build from Source

### System Requirements
- [Rust](https://rustup.rs/) (latest stable version)
- `pkg-config`, `libssl-dev` (for OpenSSL support on Linux)

### Build a debug version
```bash
cargo build
```

### Build an optimized release version
```bash
cargo build --release
```

The resulting binary will be at `target/release/query-rs`.

### Multi-Architecture Build
The project includes a `build.sh` script to build for both x86_64 and AArch64 (ARM) Linux:
```bash
bash build.sh
```
*Note: This requires the `aarch64-unknown-linux-gnu` target to be installed via rustup.*

## Running the App
Once built/installed, start the app by running:
```bash
query-rs
```
or, for development:
```bash
cargo run
```
