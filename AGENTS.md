# AGENTS.md - C# Roslyn Extension Development Guide

## Build & Test Commands

**Build WebAssembly extension:**
```bash
cargo build --target wasm32-wasip2
```

**Run all tests:**
```bash
cargo test
```

**Run a single test:**
```bash
cargo test test_debugger_version -- --exact
```

**Lint with Clippy:**
```bash
cargo clippy --target wasm32-wasip2 --all-targets
```

## Code Style Guidelines

### Imports
- Use fully-qualified paths from `zed_extension_api` (aliased as `zed`)
- Group standard library imports first, then external crates, then local modules
- Use `use` statements; avoid glob imports except in tests

### Formatting & Types
- Follow Rust 2021 edition conventions; enabled by default with rustfmt
- Return `Result<T>` for fallible operations; use `Result` from `zed_extension_api`
- Use explicit type annotations for constants (`const NAME: &str = "..."`)

### Naming Conventions
- Functions and variables: `snake_case`
- Types and constants: `UPPER_SNAKE_CASE`
- Private implementation details: prefix with `_` if unused by compiler

### Error Handling
- Use `?` operator for propagating errors wrapped in `Result`
- Error messages via `format!()` for context: `Err(format!("Context: {e}"))`
- Use `map_err()` to convert error types when needed
- Place `#[cfg(test)]` tests at file end with assertions (`assert!`, `assert_ne!`)

### Module Organization
- Separate concerns: `csharp.rs` (LSP setup), `debugger.rs` (debug support)
- Keep helper functions close to where they're used
- Document public functions and complex logic with comments

### WebAssembly-Specific
- Target: `wasm32-wasip2` (Component Model)
- Avoid platform-dependent code outside conditional blocks (`cfg!(target_os = "...")`)
- Use `zed::current_platform()` to query OS/architecture at runtime

## Project Structure
- **src/lib.rs** - Extension registration point
- **src/csharp.rs** - LSP extension implementation (initialization, solution detection)
- **src/debugger.rs** - netcoredbg debugger setup with download/extract logic
- **extension.toml** - Extension metadata for Zed plugin system
- **languages/csharp/** - Tree-sitter grammar configuration files
