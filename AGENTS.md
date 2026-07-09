# QuickEnv - AI Agent Guide

## Overview
QuickEnv is a Rust CLI tool for environment variable management and validation.
It provides validation, checking, generation, encryption, diffing, and documentation commands.

## Key Files
- `src/main.rs` — Entry point with module declarations
- `src/cli.rs` — CLI definition using clap derive
- `src/commands/` — Each subcommand implementation
- `src/envfile.rs` — .env file parser
- `src/schema.rs` — Schema rule parsing and validation
- `src/crypto.rs` — AES-256-GCM encryption/decryption
- `src/scanner.rs` — Multi-language source code scanner
- `src/reporting.rs` — Output formatting

## Build & Test
```bash
cargo build
cargo test
cargo run -- --help
```

## Conventions
- All commands follow the pattern: `ValidateArgs` struct + `execute(args) -> Result`
- Validation annotations in .env.example: @required, @secret, @default, @pattern, @allowed
- Encryption uses AES-256-GCM with Argon2 key derivation
- CLI uses clap derive for argument parsing