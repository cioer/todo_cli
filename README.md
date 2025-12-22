![GitHub Repo stars](https://img.shields.io/github/stars/skanehira/rust-cli-template?style=social)
![GitHub](https://img.shields.io/github/license/skanehira/rust-cli-template)
![GitHub all releases](https://img.shields.io/github/downloads/skanehira/rust-cli-template/total)
![GitHub CI Status](https://img.shields.io/github/actions/workflow/status/skanehira/rust-cli-template/ci.yaml?branch=main)
![GitHub Release Status](https://img.shields.io/github/v/release/skanehira/rust-cli-template)

# todoapp

A CLI app scaffold for todoapp, generated from the rust-cli-template.

## Overview

This repository is a generated Rust CLI application scaffold. It provides a
minimal yet comprehensive foundation with the following features:

- CLI argument parsing using [clap](https://github.com/clap-rs/clap) with derive
  macros
- GitHub Actions workflow for CI/CD
  - Code coverage reporting with [octocov](https://github.com/k1LoW/octocov)
  - Automatic benchmark result visualization and deployment with
    [github-action-benchmark](https://github.com/benchmark-action/github-action-benchmark)
  - Security audit checks for dependencies
  - Automated release workflow for publishing
  - Automated dependency updates with Dependabot

## Project Guardrails

- CLI parser: use `clap` only.
- Error format: `ERROR: <code> - <message>`.
- Structured output: JSON only when `--json` is provided.
- Storage: JSON file stored in user config dir; on Unix set file mode to 600, on Windows rely on user profile ACLs.

## Project Structure

Current project structure:

```
.
+-- .github/                  # GitHub Actions workflows
+-- benches/                  # Benchmark code (requires nightly Rust)
+-- crates/
|   +-- todo_core/
|   |   +-- Cargo.toml
|   |   +-- src/lib.rs
|   +-- todo_cli/
|       +-- Cargo.toml
|       +-- src/main.rs
+-- tests/
|   +-- cli_smoke.rs
+-- .gitignore
+-- .octocov.yml
+-- Cargo.toml
+-- Cargo.lock
+-- README.md
+-- rust-toolchain.toml
```

## Benchmark visualization

The benchmark results are automatically deployed to GitHub Pages for easy
visualization and performance tracking. You need to create a `gh-pages` branch
in your repository before first push.

<img width="1165" alt="image" src="https://github.com/user-attachments/assets/333631e2-dee0-48f9-bc8e-d72c583857de" />

<img width="874" alt="image" src="https://github.com/user-attachments/assets/6a07ea77-1294-422f-abd6-cb3e4281c26e" />

## Coverage

This project uses [octocov](https://github.com/k1LoW/octocov) to measure code
coverage. During CI execution, coverage reports are automatically generated and
displayed as comments on PRs or commits. The coverage history is also tracked,
allowing you to see changes over time.

The coverage reports are deployed to GitHub Pages for easy visualization.
Coverage information can also be displayed in the README as a badge.

<img width="936" alt="image" src="https://github.com/user-attachments/assets/8471d58a-06b3-4fd5-85e6-916959704c69" />

The detailed configuration for octocov is managed in the `.octocov.yml` file.

## Usage

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- C++ Build Tools (one of the following):
  - **Windows (Recommended):** [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with "Desktop development with C++" workload.
  - **Windows (Alternative):** MinGW-w64 (e.g., [w64devkit](https://github.com/skeeto/w64devkit)).
  - **Linux/macOS:** GCC or Clang (usually pre-installed).

### Setup on Windows (if using MinGW/w64devkit)

If you do not have Visual Studio Build Tools installed, you can use `w64devkit`:

1. Download [w64devkit](https://github.com/skeeto/w64devkit/releases).
2. Extract the zip file (e.g., to `C:\w64devkit`).
3. Add the `bin` folder to your PATH environment variable.
4. Configure Rust to use the GNU toolchain:
   ```bash
   rustup toolchain install stable-x86_64-pc-windows-gnu
   rustup default stable-x86_64-pc-windows-gnu
   ```

### Setup on Linux

For Linux users, you will need the standard build tools and development headers for system notifications.

1. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install Dependencies:**
   The application requires `dbus` development headers for notifications support.

   *   **Ubuntu/Debian:**
       ```bash
       sudo apt update
       sudo apt install build-essential libdbus-1-dev pkg-config
       ```

   *   **Fedora/RHEL:**
       ```bash
       sudo dnf install @development-tools dbus-devel pkgconf-pkg-config
       ```

   *   **Arch Linux:**
       ```bash
       sudo pacman -S base-devel dbus
       ```

### Running the Project

To run the CLI application:

```bash
# Run the todo_cli binary
cargo run --bin todo_cli

# View help
cargo run --bin todo_cli -- --help
```

### Build

```bash
cargo build --release --bin todo_cli
```

### Running Tests

```bash
cargo test
# Or if you have cargo-nextest installed
cargo nextest run
```

### Running Benchmarks

Benchmarks require the nightly Rust channel:

```bash
cargo +nightly bench
```

### Release Process

This template includes an automated release workflow. Follow these steps to
create a release:

1. Push a tag with your changes:
   ```bash
   git tag v0.1.0  # Replace with the appropriate version number
   git push origin v0.1.0
   ```

2. When the tag is pushed, the GitHub Actions `release.yml` workflow will
   automatically execute. This workflow:
   - Builds cross-platform binaries (Linux, macOS, Windows)
   - Creates a GitHub Release
   - Uploads binaries and changelog

The release configuration is managed in the `.github/workflows/release.yml` and
`goreleasser.yaml` files.

---

Feel free to customize this template to fit your specific needs!



