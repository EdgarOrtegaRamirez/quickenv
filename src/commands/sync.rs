use clap::Args;
use std::path::PathBuf;

use crate::envfile;
use crate::sync::{self, SyncStrategy};

#[derive(Args)]
pub struct SyncArgs {
    /// Source .env file
    pub source: PathBuf,

    /// Target .env file  
    pub target: PathBuf,

    /// Output file path (optional, defaults to stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Sync strategy: overwrite, merge, skip, add-only, update-only
    #[arg(short, long, default_value = "merge")]
    pub strategy: String,
}

pub fn execute(args: &SyncArgs) -> anyhow::Result<()> {
    let strategy = SyncStrategy::from_str(&args.strategy)?;

    let source_content = std::fs::read_to_string(&args.source)?;
    let target_content = std::fs::read_to_string(&args.target)?;

    let source_map = envfile::env_to_map(&source_content);
    let target_map = envfile::env_to_map(&target_content);

    let (result_map, result) = sync::sync(&source_map, &target_map, strategy);

    // Print summary to stderr
    eprintln!("{}", sync::format_sync_result(&result));

    // Output result to file or stdout
    let output_lines: Vec<String> = result_map
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    let output = output_lines.join("\n");

    match &args.output {
        Some(path) => std::fs::write(path, &output)?,
        None => println!("{}", output),
    }

    Ok(())
}
