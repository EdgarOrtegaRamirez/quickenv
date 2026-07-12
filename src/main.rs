#![allow(dead_code)]
mod analyzer;
mod cli;
mod commands;
mod crypto;
mod envfile;
mod format;
mod migrate;
mod reporting;
mod scanner;
mod schema;
mod sync;
mod template;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.run()
}
