use clap::Parser;

use crate::commands;

#[derive(Parser)]
#[command(
    name = "quickenv",
    version,
    about = "Environment variable management & validation CLI"
)]
pub enum Cli {
    /// Validate .env files against a schema
    Validate(commands::validate::ValidateArgs),
    /// Check for missing/extra env vars against .env.example
    Check(commands::check::CheckArgs),
    /// Generate .env.example by scanning source code
    Generate(commands::generate::GenerateArgs),
    /// Encrypt a .env file with a master key
    Encrypt(commands::encrypt::EncryptArgs),
    /// Decrypt an encrypted .env file
    Decrypt(commands::encrypt::DecryptArgs),
    /// Compare .env files across environments
    Diff(commands::diff::DiffArgs),
    /// Generate markdown documentation from .env.example
    Docs(commands::docs::DocsArgs),
    /// Audit env var usage: cross-reference code with .env files, detect secrets, find issues
    Audit(commands::audit::AuditArgs),
    /// Generate or render .env templates with variable substitution
    Template(commands::template::TemplateArgs),
    /// Apply declarative migration operations to .env files
    Migrate(commands::migrate::MigrateArgs),
    /// Synchronize .env files across environments with configurable strategies
    Sync(commands::sync::SyncArgs),
    /// Import/export .env files to/from various CI/CD formats
    Export(commands::export::ExportArgs),
}

impl Cli {
    pub fn run(&self) -> anyhow::Result<()> {
        match self {
            Cli::Validate(args) => commands::validate::execute(args),
            Cli::Check(args) => commands::check::execute(args),
            Cli::Generate(args) => commands::generate::execute(args),
            Cli::Encrypt(args) => commands::encrypt::encrypt_execute(args),
            Cli::Decrypt(args) => commands::encrypt::decrypt_execute(args),
            Cli::Diff(args) => commands::diff::execute(args),
            Cli::Docs(args) => commands::docs::execute(args),
            Cli::Audit(args) => commands::audit::execute(args),
            Cli::Template(args) => commands::template::execute(args),
            Cli::Migrate(args) => commands::migrate::execute(args),
            Cli::Sync(args) => commands::sync::execute(args),
            Cli::Export(args) => commands::export::execute(args),
        }
    }

    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}