use clap::Args;

#[derive(Args)]
pub struct DiffArgs {
    /// First .env file
    pub file_a: String,

    /// Second .env file
    pub file_b: String,

    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
}

pub fn execute(args: &DiffArgs) -> anyhow::Result<()> {
    let path_a = std::path::Path::new(&args.file_a);
    let path_b = std::path::Path::new(&args.file_b);

    if !path_a.exists() {
        anyhow::bail!("File not found: {}", args.file_a);
    }
    if !path_b.exists() {
        anyhow::bail!("File not found: {}", args.file_b);
    }

    let content_a = crate::envfile::read_env_file(path_a)?;
    let content_b = crate::envfile::read_env_file(path_b)?;

    let vars_a = crate::envfile::env_to_map(&content_a);
    let vars_b = crate::envfile::env_to_map(&content_b);

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    // Find added and changed
    for (key, value_b) in &vars_b {
        match vars_a.get(key) {
            None => added.push((key.as_str(), value_b.as_str())),
            Some(value_a) if value_a != value_b => {
                changed.push((key.as_str(), value_a.as_str(), value_b.as_str()));
            }
            _ => {}
        }
    }

    // Find removed
    for (key, value_a) in &vars_a {
        if !vars_b.contains_key(key) {
            removed.push((key.as_str(), value_a.as_str()));
        }
    }

    if args.json {
        let output = serde_json::json!({
            "file_a": args.file_a,
            "file_b": args.file_b,
            "added": added.iter().map(|(k, v)| serde_json::json!({"key": k, "value": v})).collect::<Vec<_>>(),
            "removed": removed.iter().map(|(k, v)| serde_json::json!({"key": k, "value": v})).collect::<Vec<_>>(),
            "changed": changed.iter().map(|(k, o, n)| serde_json::json!({"key": k, "old": o, "new": n})).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        crate::reporting::print_diff_report(&added, &removed, &changed, &args.file_a, &args.file_b);
    }

    Ok(())
}
