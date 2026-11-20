# Sibilant

Sibilant is a digital audio workstation (DAW) for Windows, macOS, and Linux. Sibilant uses the Rust programming language. The user interface uses the Iced library.

> [!NOTE]
> Sibilant is not complete. It does not play audio at this time.

## Goals

- Give new users and professional users one interface that is clear, fast, and easy to use.
- Arrange audio clips on a timeline, and start clips from a clip launcher.
- Host CLAP and VST3 plugins.
- Give the same quality on Windows, macOS, and Linux.

## Build Sibilant

1. Install Rust with [rustup](https://rustup.rs).
   The `rust-toolchain.toml` file selects the correct version of Rust.
2. Clone the repository:

   ```sh
   git clone https://github.com/collide2954/sibilant.git
   cd sibilant
   ```

3. Run Sibilant:

   ```sh
   cargo run --package sibilant
   ```

## Contribute

Read [CONTRIBUTING.md](CONTRIBUTING.md) before you change the code. The [decision records](docs/decisions/README.md) give the reasons for the design.

## License

The license for Sibilant is the Apache License, Version 2.0. Refer to [LICENSE](LICENSE) and [NOTICE](NOTICE).
