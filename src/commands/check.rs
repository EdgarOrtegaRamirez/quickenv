use clap::Args;
use std::collections::HashSet;

#[derive(Args)]
pub struct CheckArgs {
    /// Path to .env file
    #[arg(default_value = ".env")]
    pub env_file: String,

    /// Path to .env.example reference
    #[arg(short, long, default_value = ".env.example")]
    pub example: String,

    /// Exit with error if missing variables
    #[arg(short, long)]
    pub strict: bool,
}

pub fn execute(args: &CheckArgs) -> anyhow::Result<()> {
    let env_path = std::path::Path::new(&args.env_file);
    let example_path = std::path::Path::new(&args.example);

    if !env_path.exists() {
        anyhow::bail!("Environment file not found: {}", args.env_file);
    }
    if !example_path.exists() {
        anyhow::bail!("Example file not found: {}", args.example);
    }

    let env_content = crate::envfile::read_env_file(env_path)?;
    let example_content = crate::envfile::read_env_file(example_path)?;

    let env_vars: HashSet<String> = crate::envfile::env_to_map(&env_content).into_keys().collect();
    let example_vars: HashSet<String> = crate::envfile::env_to_map(&example_content).into_keys().collect();

    let missing: Vec<String> = {
        let mut v: Vec<String> = example_vars.difference(&env_vars).cloned().collect();
        v.sort();
        v
    };

    let extra: Vec<String> = {
        let mut v: Vec<String> = env_vars.difference(&example_vars).cloned().collect();
        v.sort();
        v
    };

    crate::reporting::print_check_results(&missing, &extra, &args.env_file, &args.example);

    if args.strict && !missing.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}