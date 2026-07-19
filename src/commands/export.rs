use clap::Args;
use std::path::PathBuf;

use crate::envfile;
use crate::format::{self, FormatType};

#[derive(Args)]
pub struct ExportArgs {
    /// Action: import or export
    #[arg(value_parser = ["import", "export"])]
    pub action: String,

    /// Input file path
    pub input: PathBuf,

    /// Output file path (optional, defaults to stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Format: json, docker-compose, kubernetes, circleci, github, dotenv
    #[arg(short, long, default_value = "json")]
    pub format: String,

    /// Source format (for import, defaults to same as --format)
    #[arg(short = 's', long)]
    pub source_format: Option<String>,
}

pub fn execute(args: &ExportArgs) -> anyhow::Result<()> {
    match args.action.as_str() {
        "import" => {
            let src_fmt = args.source_format.as_deref().unwrap_or(&args.format);
            let format_type = FormatType::from_str(src_fmt)?;
            let content = std::fs::read_to_string(&args.input)?;
            let env = format::import_env(&content, format_type)?;
            // Export as dotenv (default output format)
            let output = format::export_env(&env, FormatType::Dotenv)?;
            write_output(&output, args.output.as_ref())?;
        }
        "export" => {
            let format_type = FormatType::from_str(&args.format)?;
            let content = std::fs::read_to_string(&args.input)?;
            let env = envfile::env_to_map(&content);
            let output = format::export_env(&env, format_type)?;
            write_output(&output, args.output.as_ref())?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn write_output(data: &str, path: Option<&PathBuf>) -> anyhow::Result<()> {
    match path {
        Some(p) => std::fs::write(p, data)?,
        None => print!("{}", data),
    }
    Ok(())
}
