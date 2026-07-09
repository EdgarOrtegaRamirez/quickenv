use clap::Args;

#[derive(Args)]
pub struct EncryptArgs {
    /// Path to .env file to encrypt
    #[arg(default_value = ".env")]
    pub env_file: String,

    /// Passphrase for encryption (will prompt if not provided)
    #[arg(short, long)]
    pub passphrase: Option<String>,
}

#[derive(Args)]
pub struct DecryptArgs {
    /// Path to encrypted .env file
    #[arg(default_value = ".env.encrypted")]
    pub input_file: String,

    /// Passphrase for decryption (will prompt if not provided)
    #[arg(short, long)]
    pub passphrase: Option<String>,

    /// Output to stdout instead of file
    #[arg(short, long)]
    pub stdout: bool,

    /// Output file path (default: .env)
    #[arg(short, long)]
    pub output: Option<String>,
}

pub fn encrypt_execute(args: &EncryptArgs) -> anyhow::Result<()> {
    let passphrase = match &args.passphrase {
        Some(p) => p.clone(),
        None => rpassword::prompt_password("Enter passphrase: ")?,
    };

    let env_path = std::path::Path::new(&args.env_file);
    if !env_path.exists() {
        anyhow::bail!("File not found: {}", args.env_file);
    }

    crate::crypto::encrypt_file(env_path, &passphrase)
}

pub fn decrypt_execute(args: &DecryptArgs) -> anyhow::Result<()> {
    let passphrase = match &args.passphrase {
        Some(p) => p.clone(),
        None => rpassword::prompt_password("Enter passphrase: ")?,
    };

    let input_path = std::path::Path::new(&args.input_file);
    if !input_path.exists() {
        anyhow::bail!("File not found: {}", args.input_file);
    }

    let plaintext = crate::crypto::decrypt_file(input_path, &passphrase)?;

    if args.stdout {
        print!("{}", plaintext);
    } else {
        let output_path = args.output.as_ref().map(std::path::Path::new)
            .unwrap_or_else(|| std::path::Path::new(".env.decrypted"));
        std::fs::write(output_path, &plaintext)?;
        eprintln!("Decrypted: {} → {}", input_path.display(), output_path.display());
    }

    Ok(())
}