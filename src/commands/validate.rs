use clap::Args;

#[derive(Args)]
pub struct ValidateArgs {
    /// Path to .env file
    #[arg(default_value = ".env")]
    pub env_file: String,

    /// Path to .env.example schema file
    #[arg(short, long, default_value = ".env.example")]
    pub schema: String,

    /// Environment name for reporting
    #[arg(short, long, default_value = "default")]
    pub name: String,

    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
}

pub fn execute(args: &ValidateArgs) -> anyhow::Result<()> {
    let env_path = std::path::Path::new(&args.env_file);
    let schema_path = std::path::Path::new(&args.schema);

    if !env_path.exists() {
        anyhow::bail!("Environment file not found: {}", args.env_file);
    }
    if !schema_path.exists() {
        anyhow::bail!("Schema file not found: {}", args.schema);
    }

    let env_content = crate::envfile::read_env_file(env_path)?;
    let vars = crate::envfile::env_to_map(&env_content);
    let rules = crate::schema::load_rules_from_example(schema_path)?;
    let errors = crate::schema::validate_vars(&vars, &rules);

    if args.json {
        let output = serde_json::to_string_pretty(&errors)?;
        println!("{}", output);
    } else {
        crate::reporting::print_validation_report(&errors, &args.name);
    }

    let has_errors = errors.iter().any(|e| e.severity == crate::schema::Severity::Error);
    if has_errors {
        std::process::exit(1);
    }
    Ok(())
}