mod cli;
mod commands;
mod crypto;
mod envfile;
mod reporting;
mod scanner;
mod schema;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.run()
}