use crate::schema::{Severity, ValidationError};
use colored::*;

/// Print validation errors in a human-readable format
pub fn print_validation_report(errors: &[ValidationError], env_name: &str) {
    if errors.is_empty() {
        println!(
            "{} ✓ No issues found for '{}'",
            "OK".green().bold(),
            env_name
        );
        return;
    }

    let errors_count = errors
        .iter()
        .filter(|e| e.severity == Severity::Error)
        .count();
    let warnings_count = errors
        .iter()
        .filter(|e| e.severity == Severity::Warning)
        .count();

    println!("{} {} for '{}'", "VALIDATION".bold(), env_name, env_name,);
    println!("{}", "─".repeat(60));

    if errors_count > 0 {
        println!("  {} Errors: {}", "✗".red(), errors_count);
    }
    if warnings_count > 0 {
        println!("  {} Warnings: {}", "⚠".yellow(), warnings_count);
    }

    println!();

    for error in errors {
        let icon = match error.severity {
            Severity::Error => "✗".red(),
            Severity::Warning => "⚠".yellow(),
            Severity::Info => "ℹ".blue(),
        };
        println!(" {} {}: {}", icon, error.var_name.bold(), error.message);
    }
}

/// Print comparison of two env files
pub fn print_diff_report(
    added: &[(&str, &str)],
    removed: &[(&str, &str)],
    changed: &[(&str, &str, &str)],
    file_a: &str,
    file_b: &str,
) {
    println!("\n{}: {} vs {}", "DIFF".bold().cyan(), file_a, file_b);
    println!("{}", "─".repeat(60));

    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        println!(" {} No differences found", "✓".green());
        return;
    }

    if !added.is_empty() {
        println!("\n{} Added (+{}):", "⊕".green().bold(), added.len());
        for (key, value) in added {
            println!("  + {} = {}", key.green(), value);
        }
    }

    if !removed.is_empty() {
        println!("\n{} Removed (-{}):", "⊖".red().bold(), removed.len());
        for (key, value) in removed {
            println!("  - {} = {}", key.red(), value);
        }
    }

    if !changed.is_empty() {
        println!("\n{} Changed (~{}):", "∼".yellow().bold(), changed.len());
        for (key, old, new) in changed {
            println!("  ~ {}: {} → {}", key.yellow(), old.red(), new.green());
        }
    }
}

/// Print a summary table of env variables
pub fn print_env_table(vars: &std::collections::BTreeMap<String, String>, title: &str) {
    println!("\n{}: {} variables", title.bold().cyan(), vars.len());
    println!("{}", "─".repeat(60));

    for (key, value) in vars {
        let display_value = if value.len() > 60 {
            format!("{}...", &value[..57])
        } else {
            value.clone()
        };
        println!("  {} = {}", key.bold(), display_value);
    }
}

/// Print a generated .env.example with documentation
pub fn print_env_example(
    vars: &[(String, String, String)], /* (var_name, default_value, description) */
) {
    for (name, default, description) in vars {
        if !description.is_empty() {
            for line in description.lines() {
                println!("# {}", line);
            }
        }
        println!("{}={}", name, default);
        println!();
    }
}

/// Print check results
pub fn print_check_results(
    missing: &[String],
    extra: &[String],
    env_file: &str,
    example_file: &str,
) {
    println!(
        "\n{}: {} vs {}",
        "CHECK".bold().cyan(),
        env_file,
        example_file
    );
    println!("{}", "─".repeat(60));

    if missing.is_empty() && extra.is_empty() {
        println!(
            " {} All environment variables match the example.",
            "✓".green()
        );
        return;
    }

    if !missing.is_empty() {
        println!(
            "\n{} Missing in .env (-{}):",
            "✗".red().bold(),
            missing.len()
        );
        for v in missing {
            println!("  - {}", v.red());
        }
    }

    if !extra.is_empty() {
        println!(
            "\n{} Extra in .env (+{}):",
            "⚠".yellow().bold(),
            extra.len()
        );
        for v in extra {
            println!("  + {}", v.yellow());
        }
    }
}

/// Print a security scan finding
pub fn print_security_finding(var_name: &str, finding: &str, severity: Severity) {
    let icon = match severity {
        Severity::Error => "🔴".to_string(),
        Severity::Warning => "🟡".to_string(),
        Severity::Info => "🔵".to_string(),
    };
    println!(
        " {} [{:?}] {}: {}",
        icon,
        severity,
        var_name.bold(),
        finding
    );
}
