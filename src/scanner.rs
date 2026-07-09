use regex::Regex;
use std::collections::HashSet;
use walkdir::WalkDir;

/// Extract environment variable names from source code
pub fn scan_directory(path: &std::path::Path) -> HashSet<String> {
    let mut all_vars = HashSet::new();

    // Common patterns for env var access across languages
    let patterns: Vec<(Regex, usize)> = vec![
        // Python: os.environ['VAR'], os.environ.get('VAR'), os.getenv('VAR')
        (Regex::new(r#"os\.environ\s*\[['"]([A-Z_][A-Z0-9_]*)['"]\]"#).unwrap(), 1),
        (Regex::new(r#"os\.environ\.get\s*\(\s*['"]([A-Z_][A-Z0-9_]*)['"]"#).unwrap(), 1),
        (Regex::new(r#"os\.getenv\s*\(\s*['"]([A-Z_][A-Z0-9_]*)['"]"#).unwrap(), 1),
        // Python-dotenv: load_dotenv, str(os.environ)
        (Regex::new(r#"['"]([A-Z_][A-Z0-9_]*)['"]\s*,\s*default\s*=").unwrap(), 1),
        // Node: process.env.VAR
        (Regex::new(r#"process\.env\.([A-Z_][A-Z0-9_]*)"#).unwrap(), 1),
        (Regex::new(r#"process\.env\[['"]([A-Z_][A-Z0-9_]*)['"]\]"#).unwrap(), 1),
        // Go: os.Getenv("VAR"), os.LookupEnv("VAR")
        (Regex::new(r#"os\.Getenv\(\s*"([A-Z_][A-Z0-9_]*)"#).unwrap(), 1),
        (Regex::new(r#"os\.LookupEnv\(\s*"([A-Z_][A-Z0-9_]*)"#).unwrap(), 1),
        // Rust: std::env::var("VAR")
        (Regex::new(r#"std::env::var\(\s*"([A-Z_][A-Z0-9_]*)"#).unwrap(), 1),
        (Regex::new(r#"env::var\(\s*"([A-Z_][A-Z0-9_]*)"#).unwrap(), 1),
        // Ruby: ENV['VAR'], ENV.fetch('VAR')
        (Regex::new(r#"ENV\[['"]([A-Z_][A-Z0-9_]*)['"]\]"#).unwrap(), 1),
        (Regex::new(r#"ENV\.fetch\(['"]([A-Z_][A-Z0-9_]*)['"]"#).unwrap(), 1),
        // Shell: $VAR, ${VAR}
        (Regex::new(r#"\$\{([A-Z_][A-Z0-9_]*)\}"#).unwrap(), 1),
        // Java: System.getenv("VAR")
        (Regex::new(r#"System\.getenv\(\s*"([A-Z_][A-Z0-9_]*)"#).unwrap(), 1),
        // C#: Environment.GetEnvironmentVariable("VAR")
        (Regex::new(r#"Environment\.GetEnvironmentVariable\(\s*"([A-Z_][A-Z0-9_]*)"#).unwrap(), 1),
        // Docker: ${VAR} in Dockerfiles
        (Regex::new(r#"ENV\s+([A-Z_][A-Z0-9_]*)\s*=").unwrap(), 1),
        // Makefile: $(VAR), ${VAR}
        (Regex::new(r#"\$\(([A-Z_][A-Z0-9_]*)\)"#).unwrap(), 1),
    ];

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| !is_skip_dir(e))
    {
        if let Ok(entry) = entry {
            if entry.file_type().is_file() {
                if let Some(vars) = scan_file(entry.path(), &patterns) {
                    all_vars.extend(vars);
                }
            }
        }
    }

    all_vars
}

fn is_skip_dir(entry: &walkdir::DirEntry) -> bool {
    let skip_dirs = [
        "node_modules", ".git", "target", "build", "dist", "vendor",
        ".venv", "venv", "__pycache__", ".cache", ".terraform",
        "third_party", "third-party", "out",
    ];
    entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .map(|s| skip_dirs.contains(&s))
            .unwrap_or(false)
}

fn scan_file(path: &std::path::Path, patterns: &[(Regex, usize)]) -> Option<HashSet<String>> {
    // Skip binary files and common non-code files
    let skip_extensions = [
        "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "woff", "woff2",
        "ttf", "eot", "mp3", "mp4", "avi", "mov", "mkv", "zip", "tar", "gz",
        "bz2", "xz", "7z", "rar", "pdf", "doc", "docx", "xls", "xlsx", "o",
        "so", "dll", "dylib", "exe", "lock", "sum", "sig",
    ];

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if skip_extensions.contains(&ext) {
            return None;
        }
    }

    // Don't scan .env files themselves
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with(".env") {
            return None;
        }
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let mut vars = HashSet::new();
    let common_ignore: HashSet<&str> = [
        "HOME", "PATH", "TERM", "SHELL", "USER", "PWD", "HOSTNAME",
        "LANG", "LC_ALL", "LC_CTYPE", "LOGNAME", "UID", "GID",
    ].iter().cloned().collect();

    for (re, group_idx) in patterns {
        for cap in re.captures_iter(&content) {
            if let Some(m) = cap.get(*group_idx) {
                let var_name = m.as_str().to_string();
                if !common_ignore.contains(var_name.as_str()) {
                    vars.insert(var_name);
                }
            }
        }
    }

    if vars.is_empty() {
        None
    } else {
        Some(vars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_python_file() {
        let content = r#"
import os
db_url = os.environ['DATABASE_URL']
api_key = os.getenv('API_KEY')
log_level = os.environ.get('LOG_LEVEL')
"#;
        let patterns = vec![
            (Regex::new(r#"os\.environ\s*\[['"]([A-Z_][A-Z0-9_]*)['"]\]"#).unwrap(), 1),
            (Regex::new(r#"os\.environ\.get\s*\(\s*['"]([A-Z_][A-Z0-9_]*)['"]"#).unwrap(), 1),
            (Regex::new(r#"os\.getenv\s*\(\s*['"]([A-Z_][A-Z0-9_]*)['"]"#).unwrap(), 1),
        ];
        let mut vars = HashSet::new();
        for (re, idx) in &patterns {
            for cap in re.captures_iter(content) {
                if let Some(m) = cap.get(*idx) {
                    vars.insert(m.as_str().to_string());
                }
            }
        }
        assert!(vars.contains("DATABASE_URL"));
        assert!(vars.contains("API_KEY"));
        assert!(vars.contains("LOG_LEVEL"));
    }

    #[test]
    fn test_scan_node_file() {
        let content = r#"
const db = process.env.DATABASE_URL;
const key = process.env['API_KEY'];
"#;
        let patterns = vec![
            (Regex::new(r#"process\.env\.([A-Z_][A-Z0-9_]*)"#).unwrap(), 1),
            (Regex::new(r#"process\.env\[['"]([A-Z_][A-Z0-9_]*)['"]\]"#).unwrap(), 1),
        ];
        let mut vars = HashSet::new();
        for (re, idx) in &patterns {
            for cap in re.captures_iter(content) {
                if let Some(m) = cap.get(*idx) {
                    vars.insert(m.as_str().to_string());
                }
            }
        }
        assert!(vars.contains("DATABASE_URL"));
        assert!(vars.contains("API_KEY"));
    }
}