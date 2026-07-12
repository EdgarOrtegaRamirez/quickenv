use clap::Args;
use std::path::PathBuf;

use crate::migrate;
use crate::envfile;

#[derive(Args)]
pub struct MigrateArgs {
    /// Path to migration ops file (one action:key[:value] per line)
    pub ops_file: PathBuf,

    /// Input .env file to migrate
    pub input: PathBuf,

    /// Output file path (optional, defaults to stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

pub fn execute(args: &MigrateArgs) -> anyhow::Result<()> {
    let ops_content = std::fs::read_to_string(&args.ops_file)?;
    let ops = migrate::parse_migration_ops(&ops_content)?;

    let env_content = std::fs::read_to_string(&args.input)?;
    let env_map = envfile::env_to_map(&env_content);

    let result = migrate::migrate(&env_map, &ops);

    // Output the formatted result to stderr
    eprintln!("{}", migrate::format_migrate_result(&result));

    // Output the resulting env file
    let output_lines: Vec<String> = result.env_map.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    let output = output_lines.join("\n");

    match &args.output {
        Some(path) => std::fs::write(path, &output)?,
        None => println!("{}", output),
    }

    if !result.errors.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}