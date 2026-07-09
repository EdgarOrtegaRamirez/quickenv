# QuickEnv ⚡

> Environment variable management & validation CLI — linter, checker, encryptor, and documentation generator for your `.env` files.

QuickEnv is a fast, single-binary Rust CLI that helps teams manage environment variables across projects. It validates `.env` files against schemas, detects missing/extra variables, scans source code to generate `.env.example`, encrypts sensitive values, compares environments, and generates documentation.

## Features

| Command | Description |
|---------|-------------|
| `validate` | Validate `.env` files against a schema (`.env.example` with annotations) |
| `check` | Check for missing or extra environment variables vs `.env.example` |
| `generate` | Generate `.env.example` by scanning source code for env var usage |
| `encrypt` | Encrypt `.env` files with AES-256-GCM using a passphrase |
| `decrypt` | Decrypt encrypted `.env` files |
| `diff` | Compare two `.env` files across environments |
| `docs` | Generate Markdown documentation from `.env.example` |

## Quick Start

```bash
# Validate your .env file against .env.example
quickenv validate .env -s .env.example

# Check for missing/extra variables
quickenv check .env -e .env.example

# Generate .env.example by scanning source code
quickenv generate ./src

# Encrypt sensitive .env files
quickenv encrypt .env -p "your-passphrase"

# Compare two environments
quickenv diff .env.production .env.staging

# Generate documentation
quickenv docs .env.example > ENV_REFERENCE.md
```

## Schema Annotations

QuickEnv uses special annotations in `.env.example` comments to define validation rules:

```bash
# Database connection URL @required
DATABASE_URL=
# API key for external service @required @secret
API_KEY=*** Log level @default info @allowed debug,info,warn,error
LOG_LEVEL=info
# Optional feature flag
FEATURE_X=false
```

| Annotation | Description |
|------------|-------------|
| `@required` | Variable must be present and non-empty |
| `@secret` | Variable contains sensitive data (marked in docs) |
| `@default <value>` | Default value suggestion |
| `@pattern <regex>` | Value must match the regex pattern |
| `@allowed <v1,v2,...>` | Value must be one of the allowed values |

## Install

### From source

```bash
cargo install quickenv
```

### Build locally

```bash
git clone https://github.com/EdgarOrtegaRamirez/quickenv.git
cd quickenv
cargo build --release
./target/release/quickenv --help
```

## Source Code Scanning

The `generate` command scans source code in multiple languages for environment variable references:

- **Python**: `os.environ['VAR']`, `os.getenv('VAR')`, `os.environ.get('VAR')`
- **Node.js**: `process.env.VAR`, `process.env['VAR']`
- **Go**: `os.Getenv("VAR")`, `os.LookupEnv("VAR")`
- **Rust**: `std::env::var("VAR")`, `env::var("VAR")`
- **Ruby**: `ENV['VAR']`, `ENV.fetch('VAR')`
- **Shell**: `${VAR}`
- **Java**: `System.getenv("VAR")`
- **C#**: `Environment.GetEnvironmentVariable("VAR")`
- **Docker**: `ENV VAR=`
- **Make**: `$(VAR)`, `${VAR}`

## Encryption

QuickEnv uses AES-256-GCM with Argon2 key derivation for secure encryption:

```bash
# Encrypt
quickenv encrypt .env -p "your-passphrase"
# → Creates .env.encrypted

# Decrypt
quickenv decrypt .env.encrypted -p "your-passphrase"
# → Creates .env.decrypted or use --stdout to print
```

## Output Formats

Most commands support `-j` / `--json` for structured output:

```bash
# JSON validation output
quickenv validate -j | jq
```

## Security

See [SECURITY.md](SECURITY.md) for details on security practices and encryption.

## License

MIT — see [LICENSE](LICENSE)

## Architecture

QuickEnv is organized into modules:

- `cli.rs` — CLI argument parsing (clap)
- `envfile.rs` — `.env` file parsing
- `schema.rs` — Validation rules and schema parsing
- `crypto.rs` — AES-256-GCM encryption with Argon2 key derivation
- `scanner.rs` — Multi-language source code scanning
- `reporting.rs` — Human-readable and JSON output formatting
- `commands/` — Command implementations (validate, check, generate, encrypt, decrypt, diff, docs)