use clap::Args;

#[derive(Args)]
pub struct AuditArgs {
    /// Root directory to audit (scan for .env files and source code)
    #[arg(default_value = ".")]
    pub root_dir: String,

    /// Output format (text, json, json-pretty)
    #[arg(short, long, default_value = "text")]
    pub format: String,

    /// Exit with code 1 if any errors are found (CI mode)
    #[arg(short, long)]
    pub ci: bool,

    /// Exit with code 1 if any issues are found (strict CI mode)
    #[arg(short = 's', long)]
    pub strict: bool,

    /// Entropy threshold for secret detection (default: 4.0)
    #[arg(long, default_value = "4.0")]
    pub entropy_threshold: f64,

    /// Include test files in scan
    #[arg(long)]
    pub include_tests: bool,
}

pub fn execute(args: &AuditArgs) -> anyhow::Result<()> {
    let config = crate::analyzer::AnalyzerConfig {
        root_dir: args.root_dir.clone(),
        entropy_threshold: args.entropy_threshold,
        include_tests: args.include_tests,
        include_dot_dirs: false,
        max_file_size: 10 * 1024 * 1024,
    };

    let result = crate::analyzer::run_audit(&config)?;

    match args.format.as_str() {
        "json" => {
            print_json_output(&result, false)?;
        }
        "json-pretty" => {
            print_json_output(&result, true)?;
        }
        _ => {
            print_text_output(&result);
        }
    }

    // Determine exit code
    let error_count = result
        .issues
        .iter()
        .filter(|i| i.severity == crate::analyzer::Severity::Error)
        .count();

    if args.strict && !result.issues.is_empty() {
        std::process::exit(1);
    }

    if args.ci && error_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn print_text_output(result: &crate::analyzer::AuditResult) {
    use colored::*;

    println!();
    println!("{}", "═══════════════════════════════════════".bold());
    println!("{}", "  QUICKENV AUDIT".bold().cyan());
    println!("{}", "═══════════════════════════════════════".bold());
    println!();

    // Print summary stats
    println!(
        "{} {} files scanned, {} env files found",
        "∑".bold(),
        result.summary.files_scanned,
        result.summary.env_files_found
    );
    println!(
        "{} {} total env vars ({} defined, {} used in code)",
        "∑".bold(),
        result.summary.total_env_vars,
        result.summary.defined,
        result.summary.used_in_code
    );
    println!();

    // Print issues grouped by severity
    let errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.severity == crate::analyzer::Severity::Error)
        .collect();
    let warnings: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.severity == crate::analyzer::Severity::Warning)
        .collect();
    let infos: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.severity == crate::analyzer::Severity::Info)
        .collect();

    if !errors.is_empty() {
        println!(
            "{} Errors ({}):",
            "✗".red().bold(),
            errors.len()
        );
        println!("{}", "──────────────────────────────".red());
        for issue in &errors {
            print_issue(issue, "red");
        }
        println!();
    }

    if !warnings.is_empty() {
        println!(
            "{} Warnings ({}):",
            "⚠".yellow().bold(),
            warnings.len()
        );
        println!("{}", "──────────────────────────────".yellow());
        for issue in &warnings {
            print_issue(issue, "yellow");
        }
        println!();
    }

    if !infos.is_empty() {
        println!(
            "{} Info ({}):",
            "ℹ".blue().bold(),
            infos.len()
        );
        println!("{}", "──────────────────────────────".blue());
        for issue in &infos {
            print_issue(issue, "blue");
        }
        println!();
    }

    // Summary bar
    let total = result.issues.len();
    print!("{} {} total issues", "■".bold(), total);
    if !errors.is_empty() {
        print!(" | {} {}", "✗".red(), errors.len());
    }
    if !warnings.is_empty() {
        print!(" | {} {}", "⚠".yellow(), warnings.len());
    }
    if !infos.is_empty() {
        print!(" | {} {}", "ℹ".blue(), infos.len());
    }
    println!();

    // By language breakdown
    if !result.summary.by_language.is_empty() {
        println!();
        println!("{}", "Languages:".bold());
        let mut langs: Vec<_> = result.summary.by_language.iter().collect();
        langs.sort_by(|a, b| b.1.cmp(a.1));
        for (lang, count) in langs {
            println!("  {} {} usages", lang.cyan(), count);
        }
    }

    println!();
}

fn print_issue(issue: &crate::analyzer::Issue, color: &str) {
    use colored::*;

    let color_fn: fn(&str) -> colored::ColoredString = match color {
        "red" => |s| s.red(),
        "yellow" => |s| s.yellow(),
        "blue" => |s| s.blue(),
        _ => |s| s.normal(),
    };

    let loc = match (&issue.file, issue.line) {
        (Some(file), Some(line)) => format!("{}:{}", file, line),
        (Some(file), None) => file.clone(),
        _ => "".to_string(),
    };

    println!("    [{}] {}", color_fn(&issue.issue_type.to_string()), issue.var_name.bold());
    println!("           {} {}", color_fn("→"), issue.message);
    if !loc.is_empty() {
        println!("           {} {}", "at".dimmed(), loc.dimmed());
    }
    if let Some(ctx) = &issue.context {
        println!("           {} {}", "```".dimmed(), ctx.dimmed());
    }
    println!();
}

fn print_json_output(result: &crate::analyzer::AuditResult, pretty: bool) -> anyhow::Result<()> {
    use std::collections::BTreeMap;

    let mut output = BTreeMap::new();
    output.insert("summary".to_string(), serde_json::to_value(&result.summary)?);

    let issues: Vec<BTreeMap<String, serde_json::Value>> = result
        .issues
        .iter()
        .map(|i| {
            let mut m = BTreeMap::new();
            m.insert("type".to_string(), serde_json::Value::String(i.issue_type.to_string()));
            m.insert("var_name".to_string(), serde_json::Value::String(i.var_name.clone()));
            m.insert("severity".to_string(), serde_json::Value::String(i.severity.to_string()));
            m.insert("message".to_string(), serde_json::Value::String(i.message.clone()));
            if let Some(file) = &i.file {
                m.insert("file".to_string(), serde_json::Value::String(file.clone()));
            }
            if let Some(line) = i.line {
                m.insert("line".to_string(), serde_json::Value::Number(line.into()));
            }
            if let Some(lang) = &i.language {
                m.insert("language".to_string(), serde_json::Value::String(lang.clone()));
            }
            m
        })
        .collect();
    output.insert("issues".to_string(), serde_json::to_value(&issues)?);

    if pretty {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", serde_json::to_string(&output)?);
    }

    Ok(())
}
