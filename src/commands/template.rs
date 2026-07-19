use clap::Args;
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::template;

#[derive(Args)]
pub struct TemplateArgs {
    /// Action to perform: generate, render, show
    #[arg(value_parser = ["generate", "render", "show"])]
    pub action: String,

    /// Input .env file path (for generate/show) or template file path (for render)
    pub input: PathBuf,

    /// Output file path (optional, defaults to stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Variable values for render action: KEY=value (can be specified multiple times)
    #[arg(short, long, value_parser = parse_key_val)]
    pub set: Vec<(String, String)>,
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

pub fn execute(args: &TemplateArgs) -> anyhow::Result<()> {
    match args.action.as_str() {
        "generate" => {
            let content = std::fs::read_to_string(&args.input)?;
            let tmpl = template::generate_from_env(&content);
            let formatted = template::format_template(&tmpl, true);
            write_output(&formatted, args.output.as_ref())?;
        }
        "render" => {
            let content = std::fs::read_to_string(&args.input)?;
            let tmpl = template::parse_template(&content);
            let values: BTreeMap<String, String> = args.set.iter().cloned().collect();
            let rendered = template::render_template(&tmpl, &values);
            write_output(&rendered, args.output.as_ref())?;
        }
        "show" => {
            let content = std::fs::read_to_string(&args.input)?;
            let tmpl = template::parse_template(&content);
            let formatted = template::format_template(&tmpl, true);
            write_output(&formatted, args.output.as_ref())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_val() {
        let (k, v) = parse_key_val("KEY=value").unwrap();
        assert_eq!(k, "KEY");
        assert_eq!(v, "value");
    }

    #[test]
    fn test_parse_key_val_no_equals() {
        assert!(parse_key_val("invalid").is_err());
    }
}
