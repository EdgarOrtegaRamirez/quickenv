use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use walkdir::WalkDir;

/// Severity level for audit issues
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// An issue found during audit
#[derive(Debug, Clone)]
pub struct Issue {
    pub issue_type: IssueType,
    pub var_name: String,
    pub message: String,
    pub severity: Severity,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub language: Option<String>,
    pub context: Option<String>,
}

/// Types of issues detected during audit
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueType {
    MissingEnv,
    UnusedEnv,
    EmptyValue,
    PotentialSecret,
    DuplicateDef,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::MissingEnv => write!(f, "missing-env"),
            IssueType::UnusedEnv => write!(f, "unused-env"),
            IssueType::EmptyValue => write!(f, "empty-value"),
            IssueType::PotentialSecret => write!(f, "potential-secret"),
            IssueType::DuplicateDef => write!(f, "duplicate-def"),
        }
    }
}

/// A single env var usage found in code
#[derive(Debug, Clone)]
pub struct CodeUsage {
    pub var_name: String,
    pub file: String,
    pub line: usize,
    pub language: String,
    pub context: String,
    pub has_default: bool,
}

/// A single env var definition from a .env file
#[derive(Debug, Clone)]
pub struct EnvDefinition {
    pub var_name: String,
    pub value: String,
    pub file: String,
    pub line: usize,
}

/// Summary of the audit
#[derive(Debug, Clone, Serialize)]
pub struct AuditSummary {
    pub total_env_vars: usize,
    pub defined: usize,
    pub used_in_code: usize,
    pub missing: usize,
    pub unused: usize,
    pub empty_values: usize,
    pub potential_secrets: usize,
    pub duplicates: usize,
    pub files_scanned: usize,
    pub env_files_found: usize,
    pub by_language: HashMap<String, usize>,
    pub by_severity: HashMap<String, usize>,
}

/// Result of an audit
#[derive(Debug, Clone)]
pub struct AuditResult {
    pub issues: Vec<Issue>,
    pub summary: AuditSummary,
    pub code_usages: Vec<CodeUsage>,
    pub env_definitions: Vec<EnvDefinition>,
}

/// Configuration for the analyzer
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    pub root_dir: String,
    pub entropy_threshold: f64,
    pub include_tests: bool,
    pub include_dot_dirs: bool,
    pub max_file_size: u64, // bytes
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            root_dir: ".".to_string(),
            entropy_threshold: 4.0,
            include_tests: false,
            include_dot_dirs: false,
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Language pattern definitions for env var detection
struct LangPatterns {
    get_patterns: Vec<Regex>,
    set_patterns: Vec<Regex>,
    def_patterns: Vec<Regex>,
    file_exts: Vec<&'static str>,
    language_name: &'static str,
}

fn build_language_patterns() -> Vec<LangPatterns> {
    vec![
        // Python
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"os\.environ\.get\(\s*['"]([^'"]+)['"]"#).unwrap(),
                Regex::new(r#"os\.environ\[\s*['"]([^'"]+)['"]\s*\]"#).unwrap(),
                Regex::new(r#"os\.getenv\(\s*['"]([^'"]+)['"]"#).unwrap(),
                Regex::new(r#"environ\.get\(\s*['"]([^'"]+)['"]"#).unwrap(),
                Regex::new(r#"environ\[\s*['"]([^'"]+)['"]\s*\]"#).unwrap(),
                Regex::new(r#"env\.get\(\s*['"]([^'"]+)['"]"#).unwrap(),
            ],
            set_patterns: vec![
                Regex::new(r#"os\.environ\[\s*['"]([^'"]+)['"]\s*\]\s*="#).unwrap(),
                Regex::new(r#"os\.environ\.update\(\s*\{['"]([^'"]+)['"]"#).unwrap(),
                Regex::new(r#"os\.putenv\(\s*['"]([^'"]+)['"]"#).unwrap(),
                Regex::new(r#"os\.environ\.setdefault\(\s*['"]([^'"]+)['"]"#).unwrap(),
            ],
            def_patterns: vec![
                Regex::new(r#"os\.environ\.get\(\s*['"]([^'"]+)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#)
                    .unwrap(),
                Regex::new(r#"os\.getenv\(\s*['"]([^'"]+)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#)
                    .unwrap(),
            ],
            file_exts: vec![".py"],
            language_name: "python",
        },
        // JavaScript/TypeScript
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"process\.env\.([A-Za-z_][A-Za-z0-9_]*)"#).unwrap(),
                Regex::new(r#"process\.env\[\s*['"]([^'"]+)['"]\s*\]"#).unwrap(),
                Regex::new(r#"import\.meta\.env\.([A-Za-z_][A-Za-z0-9_]*)"#).unwrap(),
                Regex::new(r#"import\.meta\.env\[\s*['"]([^'"]+)['"]\s*\]"#).unwrap(),
                Regex::new(r#"Deno\.env\.get\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap(),
                Regex::new(r#"Bun\.env\.([A-Za-z_][A-Za-z0-9_]*)"#).unwrap(),
            ],
            set_patterns: vec![
                Regex::new(r#"process\.env\.([A-Za-z_][A-Za-z0-9_]*)\s*="#).unwrap(),
                Regex::new(r#"process\.env\[\s*['"]([^'"]+)['"]\s*\]\s*="#).unwrap(),
                Regex::new(r#"Deno\.env\.set\(\s*['"]([^'"]+)['"]"#).unwrap(),
            ],
            def_patterns: vec![
                Regex::new(r#"process\.env\.([A-Za-z_][A-Za-z0-9_]*)\s*\|\|\s*['"]([^'"]*)['"]"#)
                    .unwrap(),
                Regex::new(r#"process\.env\.([A-Za-z_][A-Za-z0-9_]*)\s*\?\?\s*['"]([^'"]*)['"]"#)
                    .unwrap(),
            ],
            file_exts: vec![".js", ".ts", ".jsx", ".tsx", ".mjs", ".cjs"],
            language_name: "javascript",
        },
        // Go
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"os\.Getenv\(\s*"([^"]+)"\s*\)"#).unwrap(),
                Regex::new(r#"os\.LookupEnv\(\s*"([^"]+)"\s*\)"#).unwrap(),
                Regex::new(
                    r#"viper\.(?:Get|GetString|GetInt|GetBool|GetDuration)\(\s*"([^"]+)"\s*\)"#,
                )
                .unwrap(),
            ],
            set_patterns: vec![Regex::new(r#"os\.Setenv\(\s*"([^"]+)"\s*,"#).unwrap()],
            def_patterns: vec![],
            file_exts: vec![".go"],
            language_name: "go",
        },
        // Ruby
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"ENV\[\s*['"]([^'"]+)['"]\s*\]"#).unwrap(),
                Regex::new(r#"ENV\.fetch\(\s*['"]([^'"]+)['"]"#).unwrap(),
                Regex::new(r#"ENV\.dig\(\s*['"]([^'"]+)['"]"#).unwrap(),
            ],
            set_patterns: vec![
                Regex::new(r#"ENV\[\s*['"]([^'"]+)['"]\s*\]\s*="#).unwrap(),
                Regex::new(r#"ENV\.store\(\s*['"]([^'"]+)['"]"#).unwrap(),
            ],
            def_patterns: vec![Regex::new(
                r#"ENV\.fetch\(\s*['"]([^'"]+)['"]\s*,\s*['"]([^'"]*)['"]\s*\)"#,
            )
            .unwrap()],
            file_exts: vec![".rb", ".rake"],
            language_name: "ruby",
        },
        // Rust
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"std::env::var\(\s*"([^"]+)"\s*\)"#).unwrap(),
                Regex::new(r#"std::env::var_os\(\s*"([^"]+)"\s*\)"#).unwrap(),
                Regex::new(r#"env::var\(\s*"([^"]+)"\s*\)"#).unwrap(),
                Regex::new(r#"env::var_os\(\s*"([^"]+)"\s*\)"#).unwrap(),
                Regex::new(r#"dotenv::var\(\s*"([^"]+)"\s*\)"#).unwrap(),
            ],
            set_patterns: vec![
                Regex::new(r#"std::env::set_var\(\s*"([^"]+)"\s*,"#).unwrap(),
                Regex::new(r#"env::set_var\(\s*"([^"]+)"\s*,"#).unwrap(),
            ],
            def_patterns: vec![],
            file_exts: vec![".rs"],
            language_name: "rust",
        },
        // Shell scripts
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"\$\{([A-Za-z_][A-Za-z0-9_]*)[:\-]([^}]*)\}"#).unwrap(),
                Regex::new(r#"\$\{?([A-Za-z_][A-Za-z0-9_]*)\}?"#).unwrap(),
            ],
            set_patterns: vec![Regex::new(r#"export\s+([A-Za-z_][A-Za-z0-9_]*)\s*="#).unwrap()],
            def_patterns: vec![Regex::new(r#"\$\{([A-Za-z_][A-Za-z0-9_]*)[:\-]([^}]*)\}"#).unwrap()],
            file_exts: vec![".sh", ".bash", ".zsh"],
            language_name: "shell",
        },
        // Java
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"System\.getenv\(\s*"([^"]+)"\s*\)"#).unwrap(),
                Regex::new(r#"System\.getenv\(\)\.get\(\s*"([^"]+)"\s*\)"#).unwrap(),
            ],
            set_patterns: vec![],
            def_patterns: vec![],
            file_exts: vec![".java", ".kt", ".kts"],
            language_name: "java",
        },
        // C#
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"Environment\.GetEnvironmentVariable\(\s*"([^"]+)"\s*\)"#).unwrap(),
                Regex::new(r#"Environment\.GetEnvironmentVariable\(\s*"([^"]+)"\s*,"#).unwrap(),
            ],
            set_patterns: vec![Regex::new(
                r#"Environment\.SetEnvironmentVariable\(\s*"([^"]+)"\s*,"#,
            )
            .unwrap()],
            def_patterns: vec![],
            file_exts: vec![".cs", ".csx"],
            language_name: "csharp",
        },
        // Dockerfile
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"\$\{([A-Z_][A-Z0-9_]*)\}"#).unwrap(),
                Regex::new(r#"\$([A-Z_][A-Z0-9_]*)"#).unwrap(),
            ],
            set_patterns: vec![
                Regex::new(r#"ENV\s+([A-Z_][A-Z0-9_]*)\s*="#).unwrap(),
                Regex::new(r#"ENV\s+([A-Z_][A-Z0-9_]*)\s+"#).unwrap(),
            ],
            def_patterns: vec![],
            file_exts: vec!["Dockerfile", ".dockerfile"],
            language_name: "docker",
        },
        // Makefile
        LangPatterns {
            get_patterns: vec![
                Regex::new(r#"\$\(([A-Z_][A-Z0-9_]*)\)"#).unwrap(),
                Regex::new(r#"\$\{([A-Z_][A-Z0-9_]*)\}"#).unwrap(),
            ],
            set_patterns: vec![Regex::new(r#"^([A-Z_][A-Z0-9_]*)\s*[:+?]?="#).unwrap()],
            def_patterns: vec![],
            file_exts: vec!["Makefile", "makefile", "GNUmakefile", ".mk"],
            language_name: "makefile",
        },
    ]
}

/// Common system environment variables to ignore
fn is_common_env_var(name: &str) -> bool {
    let common: HashSet<&str> = [
        "PATH",
        "HOME",
        "USER",
        "SHELL",
        "TERM",
        "LANG",
        "LC_ALL",
        "PWD",
        "OLDPWD",
        "SHLVL",
        "LOGNAME",
        "HOSTNAME",
        "DISPLAY",
        "TMPDIR",
        "EDITOR",
        "VISUAL",
        "PAGER",
        "XDG_RUNTIME_DIR",
        "XDG_DATA_HOME",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "BASH_VERSION",
        "ZSH_VERSION",
        "RUST_LOG",
        "RUST_BACKTRACE",
        "NODE_ENV",
        "NPM_CONFIG_LOGLEVEL",
    ]
    .iter()
    .cloned()
    .collect();
    common.contains(&name.to_uppercase().as_str())
}

/// Check if a variable name is a valid env var (starts with letter or underscore)
fn is_valid_env_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    // Filter out common shell special variables
    let special = ["$", "#", "?", "!", "0", "@", "*", "-", "_"];
    if special.contains(&name) {
        return false;
    }
    let first = name.chars().next().unwrap();
    first == '_' || first.is_ascii_alphabetic()
}

/// Check if a directory should be skipped
fn is_skip_dir(entry: &walkdir::DirEntry) -> bool {
    let skip_dirs: HashSet<&str> = [
        "node_modules",
        ".git",
        "target",
        "build",
        "dist",
        "vendor",
        ".venv",
        "venv",
        "__pycache__",
        ".cache",
        ".terraform",
        "third_party",
        "third-party",
        "out",
        "coverage",
    ]
    .iter()
    .cloned()
    .collect();
    entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .map(|s| skip_dirs.contains(s))
            .unwrap_or(false)
}

/// Check if a file is a test file
fn is_test_file(path: &std::path::Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.contains("_test.")
        || name.contains(".test.")
        || name.contains(".spec.")
        || name.starts_with("test_")
        || name.ends_with("_test.rs")
        || name.ends_with("_test.go")
}

/// Check if file extension matches any known language pattern
fn get_language_for_file(path: &std::path::Path, languages: &[LangPatterns]) -> Option<usize> {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    for (idx, lang) in languages.iter().enumerate() {
        for lang_ext in &lang.file_exts {
            if *lang_ext == file_name
                || *lang_ext == format!(".{}", ext)
                || ext == lang_ext.trim_start_matches('.')
            {
                return Some(idx);
            }
        }
    }
    None
}

/// Shannon entropy calculation
fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut freq: HashMap<char, usize> = HashMap::new();
    let length = s.chars().count();
    if length == 0 {
        return 0.0;
    }
    for c in s.chars() {
        *freq.entry(c).or_insert(0) += 1;
    }
    let mut entropy = 0.0;
    let len_f64 = length as f64;
    for &count in freq.values() {
        let p = count as f64 / len_f64;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// Check if a value looks like a potential secret
fn is_potential_secret(name: &str, value: &str) -> bool {
    if value.is_empty() || value.len() < 8 {
        return false;
    }

    // Skip variable references like ${VAR} or $VAR
    if value.starts_with('$') || value.starts_with("${") {
        return false;
    }

    // Skip placeholder values
    let lower_value = value.to_lowercase();
    let placeholders = [
        "changeme",
        "your-",
        "xxx",
        "placeholder",
        "example",
        "replace",
        "insert",
        "enter",
        "todo",
        "fixme",
    ];
    for p in &placeholders {
        if lower_value.contains(p) {
            return false;
        }
    }

    // Skip URLs (scheme://...)
    if value.contains("://") {
        return false;
    }

    // Check name for secret-like patterns
    let upper_name = name.to_uppercase();
    let secret_names = [
        "SECRET",
        "KEY",
        "TOKEN",
        "PASSWORD",
        "PASSWD",
        "CREDENTIAL",
        "AUTH",
        "API_KEY",
        "APIKEY",
        "PRIVATE",
        "SIGNING",
        "ENCRYPTION",
        "CERT",
        "CERTIFICATE",
        "ACCESS_KEY",
        "SECRET_KEY",
    ];
    for sn in &secret_names {
        if upper_name.contains(sn) {
            return true;
        }
    }

    // Check value entropy (high entropy = likely a secret)
    if shannon_entropy(value) > 4.0 && value.len() >= 16 {
        return true;
    }

    // Check for base64-like patterns
    let base64_re = Regex::new(r"^[A-Za-z0-9+/]{16,}={0,2}$").unwrap();
    if base64_re.is_match(value) {
        return true;
    }

    // Check for hex-like patterns (API keys are often hex)
    let hex_re = Regex::new(r"^[0-9a-f]{32,}$").unwrap();
    if hex_re.is_match(value.to_lowercase().as_str()) {
        return true;
    }

    false
}

/// Parse .env file content into definitions
fn parse_env_file(content: &str, file_path: &str) -> Vec<EnvDefinition> {
    let mut defs = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let value_raw = trimmed[eq_pos + 1..].trim();
            let value = value_raw
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| {
                    value_raw
                        .strip_prefix('\'')
                        .and_then(|s| s.strip_suffix('\''))
                })
                .unwrap_or(value_raw)
                .to_string();
            if !key.is_empty() {
                defs.push(EnvDefinition {
                    var_name: key,
                    value,
                    file: file_path.to_string(),
                    line: i + 1,
                });
            }
        }
    }
    defs
}

/// Scan a single file for env var usage
fn scan_file(path: &std::path::Path, lang: &LangPatterns) -> anyhow::Result<Vec<CodeUsage>> {
    let content = std::fs::read_to_string(path)?;
    let file_path = path.to_string_lossy().to_string();
    let mut results = Vec::new();
    // Track which (var_name, line) pairs already have default entries
    // to avoid duplicating from get_patterns
    use std::collections::HashSet;
    let mut has_default_set: HashSet<(String, usize)> = HashSet::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_no = line_num + 1;

        // Check default patterns FIRST — these include both name and default value
        for pattern in &lang.def_patterns {
            for cap in pattern.captures_iter(line) {
                if let Some(m) = cap.get(1) {
                    let name = m.as_str().to_string();
                    if is_valid_env_name(&name) && !is_common_env_var(&name) {
                        let has_default = cap.get(2).is_some();
                        has_default_set.insert((name.clone(), line_no));
                        results.push(CodeUsage {
                            var_name: name,
                            file: file_path.clone(),
                            line: line_no,
                            language: lang.language_name.to_string(),
                            context: line.trim().to_string(),
                            has_default,
                        });
                    }
                }
            }
        }

        // Check get patterns — skip variables already handled by default patterns
        for pattern in &lang.get_patterns {
            for cap in pattern.captures_iter(line) {
                if let Some(m) = cap.get(1) {
                    let name = m.as_str().to_string();
                    if is_valid_env_name(&name) && !is_common_env_var(&name) {
                        // Skip if we already found this var on this line via a default pattern
                        if has_default_set.contains(&(name.clone(), line_no)) {
                            continue;
                        }
                        results.push(CodeUsage {
                            var_name: name,
                            file: file_path.clone(),
                            line: line_no,
                            language: lang.language_name.to_string(),
                            context: line.trim().to_string(),
                            has_default: false,
                        });
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Run the full audit
pub fn run_audit(config: &AnalyzerConfig) -> anyhow::Result<AuditResult> {
    let root = std::path::Path::new(&config.root_dir);
    if !root.exists() {
        anyhow::bail!("Directory not found: {}", config.root_dir);
    }

    let languages = build_language_patterns();
    let mut code_usages = Vec::new();
    let mut env_definitions = Vec::new();
    let mut files_scanned = 0;

    // Step 1: Scan code files for env var usage
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_str().unwrap_or("");
                if config.include_dot_dirs {
                    !matches!(
                        name,
                        "node_modules"
                            | ".git"
                            | "target"
                            | "build"
                            | "dist"
                            | "vendor"
                            | ".venv"
                            | "venv"
                            | "__pycache__"
                            | ".terraform"
                            | "third_party"
                            | "third-party"
                            | "out"
                            | "coverage"
                    )
                } else {
                    !is_skip_dir(e)
                }
            } else {
                true
            }
        })
        .flatten()
    {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }

        // Skip test files if not configured
        if !config.include_tests && is_test_file(path) {
            continue;
        }

        // Check file size
        if let Ok(meta) = entry.metadata() {
            if meta.len() > config.max_file_size {
                continue;
            }
        }

        // Skip .env files themselves
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with(".env") {
                // Parse as .env file
                if let Ok(content) = std::fs::read_to_string(path) {
                    let file_path = path.to_string_lossy().to_string();
                    let defs = parse_env_file(&content, &file_path);
                    env_definitions.extend(defs);
                }
                continue;
            }
        }

        // Skip binary files
        let skip_ext = [
            "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "woff", "woff2", "ttf", "eot", "mp3",
            "mp4", "avi", "mov", "mkv", "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "pdf", "doc",
            "docx", "xls", "xlsx", "o", "so", "dll", "dylib", "exe", "lock", "sum", "sig",
        ];
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if skip_ext.contains(&ext) {
                continue;
            }
        }

        // Find matching language
        if let Some(lang_idx) = get_language_for_file(path, &languages) {
            match scan_file(path, &languages[lang_idx]) {
                Ok(usages) => {
                    if !usages.is_empty() {
                        code_usages.extend(usages);
                        files_scanned += 1;
                    }
                }
                Err(_) => continue,
            }
        }
    }

    // Step 2: Build env map — cross-reference code usage vs definitions
    let mut env_map: BTreeMap<String, (Vec<CodeUsage>, Vec<EnvDefinition>)> = BTreeMap::new();

    for usage in &code_usages {
        env_map
            .entry(usage.var_name.clone())
            .or_insert_with(|| (Vec::new(), Vec::new()))
            .0
            .push(usage.clone());
    }

    for def in &env_definitions {
        env_map
            .entry(def.var_name.clone())
            .or_insert_with(|| (Vec::new(), Vec::new()))
            .1
            .push(def.clone());
    }

    // Step 3: Detect issues
    let mut issues = Vec::new();

    // Check for duplicate definitions in the same file
    let mut def_seen: HashMap<(&str, &str), usize> = HashMap::new();
    for def in &env_definitions {
        let key = (def.var_name.as_str(), def.file.as_str());
        if let Some(&prev_line) = def_seen.get(&key) {
            issues.push(Issue {
                issue_type: IssueType::DuplicateDef,
                var_name: def.var_name.clone(),
                message: format!(
                    "Defined multiple times in {} (lines {} and {})",
                    def.file, prev_line, def.line
                ),
                severity: Severity::Warning,
                file: Some(def.file.clone()),
                line: Some(def.line),
                language: None,
                context: None,
            });
        }
        def_seen.insert(key, def.line);
    }

    for (var_name, (usages, defs)) in &env_map {
        // Missing env var: used in code but not defined in .env
        if !usages.is_empty() && defs.is_empty() {
            let has_default = usages.iter().any(|u| u.has_default);
            let severity = if has_default {
                Severity::Warning
            } else {
                Severity::Error
            };
            let message = if has_default {
                "Used in code with default value but not defined in any .env file".to_string()
            } else {
                "Used in code but not defined in any .env file — application may fail at runtime"
                    .to_string()
            };
            let first_usage = &usages[0];
            issues.push(Issue {
                issue_type: IssueType::MissingEnv,
                var_name: var_name.clone(),
                message,
                severity,
                file: Some(first_usage.file.clone()),
                line: Some(first_usage.line),
                language: Some(first_usage.language.clone()),
                context: Some(first_usage.context.clone()),
            });
        }

        // Unused env var: defined in .env but not used in code
        if defs.is_empty() && !usages.is_empty() {
            continue; // Already handled above
        }
        if !defs.is_empty() && usages.is_empty() {
            let first_def = &defs[0];
            // Skip common env vars
            if !is_common_env_var(var_name) {
                issues.push(Issue {
                    issue_type: IssueType::UnusedEnv,
                    var_name: var_name.clone(),
                    message: "Defined in .env but not referenced in any source code".to_string(),
                    severity: Severity::Warning,
                    file: Some(first_def.file.clone()),
                    line: Some(first_def.line),
                    language: None,
                    context: None,
                });
            }
        }

        // Empty value check
        for def in defs {
            if def.value.is_empty() {
                issues.push(Issue {
                    issue_type: IssueType::EmptyValue,
                    var_name: var_name.clone(),
                    message: "Defined with empty value in .env file".to_string(),
                    severity: Severity::Info,
                    file: Some(def.file.clone()),
                    line: Some(def.line),
                    language: None,
                    context: None,
                });
            }
        }

        // Potential secret detection
        for def in defs {
            if is_potential_secret(var_name, &def.value) {
                issues.push(Issue {
                    issue_type: IssueType::PotentialSecret,
                    var_name: var_name.clone(),
                    message: "Value looks like a secret or API key — ensure it's not committed to version control".to_string(),
                    severity: Severity::Warning,
                    file: Some(def.file.clone()),
                    line: Some(def.line),
                    language: None,
                    context: None,
                });
            }
        }
    }

    // Step 4: Build summary
    let mut summary = AuditSummary {
        total_env_vars: env_map.len(),
        defined: env_definitions.len(),
        used_in_code: code_usages.len(),
        missing: 0,
        unused: 0,
        empty_values: 0,
        potential_secrets: 0,
        duplicates: 0,
        files_scanned,
        env_files_found: env_definitions
            .iter()
            .map(|d| d.file.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        by_language: HashMap::new(),
        by_severity: HashMap::new(),
    };

    for issue in &issues {
        match issue.issue_type {
            IssueType::MissingEnv => summary.missing += 1,
            IssueType::UnusedEnv => summary.unused += 1,
            IssueType::EmptyValue => summary.empty_values += 1,
            IssueType::PotentialSecret => summary.potential_secrets += 1,
            IssueType::DuplicateDef => summary.duplicates += 1,
        }
        *summary
            .by_severity
            .entry(issue.severity.to_string())
            .or_insert(0) += 1;
    }

    // Count by language
    for usage in &code_usages {
        *summary
            .by_language
            .entry(usage.language.clone())
            .or_insert(0) += 1;
    }

    Ok(AuditResult {
        issues,
        summary,
        code_usages,
        env_definitions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_entropy() {
        // Low entropy: repeated characters
        let low = shannon_entropy("aaaaaa");
        // High entropy: random-looking string
        let high = shannon_entropy("aB3dEfGh1JkLmNoPqRsTuVwXyZ0123456789");
        assert!(low < high, "Low entropy should be less than high entropy");
        assert!(high > 4.0, "Random string should have high entropy");
    }

    #[test]
    fn test_is_potential_secret_by_name() {
        assert!(is_potential_secret("API_KEY", "sk-123...cdef"));
        assert!(is_potential_secret("DB_PASSWORD", "supersecret123!"));
        assert!(is_potential_secret("SECRET_TOKEN", "abcdef1234567890"));
        assert!(!is_potential_secret("DATABASE_URL", "localhost"));
        assert!(!is_potential_secret("LOG_LEVEL", "debug"));
        // Placeholder values should not be flagged
        assert!(!is_potential_secret("API_KEY", "changeme"));
    }

    #[test]
    fn test_is_potential_secret_by_entropy() {
        // High entropy value >= 16 chars
        assert!(is_potential_secret(
            "CONFIG_VAR",
            "aB3dEfGh1JkLmNoPqRsTuVwXyZ0123456789"
        ));
        // Short value (< 8 chars)
        assert!(!is_potential_secret("CONFIG_VAR", "abc"));
    }

    #[test]
    fn test_is_potential_secret_by_hex() {
        // 32+ hex chars
        let hex_key = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0";
        assert!(is_potential_secret("SECRET", hex_key));
    }

    #[test]
    fn test_is_valid_env_name() {
        assert!(is_valid_env_name("DATABASE_URL"));
        assert!(is_valid_env_name("_SECRET"));
        assert!(!is_valid_env_name(""));
        assert!(!is_valid_env_name("$"));
        assert!(!is_valid_env_name("123ABC"));
    }

    #[test]
    fn test_parse_env_file() {
        let content =
            "DATABASE_URL=postgres://localhost:5432/mydb\n# comment\nAPI_KEY=secret123\nEMPTY=\n";
        let defs = parse_env_file(content, ".env");
        assert_eq!(defs.len(), 3);
        assert_eq!(defs[0].var_name, "DATABASE_URL");
        assert_eq!(defs[0].value, "postgres://localhost:5432/mydb");
        assert_eq!(defs[1].var_name, "API_KEY");
        assert_eq!(defs[1].value, "secret123");
        assert_eq!(defs[2].var_name, "EMPTY");
        assert_eq!(defs[2].value, "");
    }

    #[test]
    fn test_parse_env_file_quoted() {
        let content = "NAME=\"John Doe\"\nSINGLE='value'";
        let defs = parse_env_file(content, ".env");
        assert_eq!(defs[0].value, "John Doe");
        assert_eq!(defs[1].value, "value");
    }

    #[test]
    fn test_scan_python_code() {
        let content = "import os\nurl = os.environ.get('DATABASE_URL')\nkey = os.getenv('API_KEY')\nlevel = os.environ.get('LOG_LEVEL', 'info')\n";
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("config.py");
        std::fs::write(&file_path, content).unwrap();

        let languages = build_language_patterns();
        let lang_idx = get_language_for_file(&file_path, &languages).unwrap();
        let results = scan_file(&file_path, &languages[lang_idx]).unwrap();

        let names: Vec<String> = results.iter().map(|r| r.var_name.clone()).collect();
        assert!(names.contains(&"DATABASE_URL".to_string()));
        assert!(names.contains(&"API_KEY".to_string()));
        assert!(names.contains(&"LOG_LEVEL".to_string()));

        // LOG_LEVEL has a default
        let log_level = results.iter().find(|r| r.var_name == "LOG_LEVEL").unwrap();
        assert!(log_level.has_default);
    }

    #[test]
    fn test_scan_node_code() {
        let content = "const db = process.env.DATABASE_URL;\nconst key = process.env['API_KEY'];\nconst port = process.env.PORT || '3000';\n";
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("config.ts");
        std::fs::write(&file_path, content).unwrap();

        let languages = build_language_patterns();
        let lang_idx = get_language_for_file(&file_path, &languages).unwrap();
        let results = scan_file(&file_path, &languages[lang_idx]).unwrap();

        let names: Vec<String> = results.iter().map(|r| r.var_name.clone()).collect();
        assert!(names.contains(&"DATABASE_URL".to_string()));
        assert!(names.contains(&"API_KEY".to_string()));
        assert!(names.contains(&"PORT".to_string()));
    }

    #[test]
    fn test_scan_go_code() {
        let content = r#"package main
import "os"
func main() {
    db := os.Getenv("DATABASE_URL")
    key, ok := os.LookupEnv("API_KEY")
}"#;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("main.go");
        std::fs::write(&file_path, content).unwrap();

        let languages = build_language_patterns();
        let lang_idx = get_language_for_file(&file_path, &languages).unwrap();
        let results = scan_file(&file_path, &languages[lang_idx]).unwrap();

        let names: Vec<String> = results.iter().map(|r| r.var_name.clone()).collect();
        assert!(names.contains(&"DATABASE_URL".to_string()));
        assert!(names.contains(&"API_KEY".to_string()));
    }

    #[test]
    fn test_full_audit() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create a .env file
        std::fs::write(
            root.join(".env"),
            "DATABASE_URL=postgres://localhost:5432/mydb\nAPI_KEY=sk-secret123\nUNUSED_VAR=hello\nEMPTY_VAR=\n",
        )
        .unwrap();

        // Create a .env.example
        std::fs::write(
            root.join(".env.example"),
            "DATABASE_URL=\nAPI_KEY=\nLOG_LEVEL=debug\n",
        )
        .unwrap();

        // Create a Python file that uses some vars
        std::fs::write(
            root.join("app.py"),
            "import os\nurl = os.environ.get('DATABASE_URL')\nkey = os.getenv('API_KEY')\nlevel = os.environ.get('LOG_LEVEL', 'info')\nmissing_var = os.getenv('MISSING_VAR')\n",
        )
        .unwrap();

        let config = AnalyzerConfig {
            root_dir: root.to_string_lossy().to_string(),
            entropy_threshold: 4.0,
            include_tests: false,
            include_dot_dirs: false,
            max_file_size: 10 * 1024 * 1024,
        };

        let result = run_audit(&config).unwrap();

        // Should have issues: missing (MISSING_VAR), unused (UNUSED_VAR), empty (EMPTY_VAR), potential secret (API_KEY)
        assert!(
            result.issues.len() >= 4,
            "Expected at least 4 issues, got {}",
            result.issues.len()
        );

        let issue_types: Vec<IssueType> =
            result.issues.iter().map(|i| i.issue_type.clone()).collect();
        assert!(issue_types.contains(&IssueType::MissingEnv));
        assert!(issue_types.contains(&IssueType::UnusedEnv));
        assert!(issue_types.contains(&IssueType::EmptyValue));
        assert!(issue_types.contains(&IssueType::PotentialSecret));

        // Summary checks
        assert!(result.summary.files_scanned >= 1);
        assert!(result.summary.env_files_found >= 2);
        assert!(result.summary.missing >= 1);
        assert!(result.summary.unused >= 1);
        assert!(result.summary.empty_values >= 1);
        assert!(result.summary.potential_secrets >= 1);
    }

    #[test]
    fn test_language_detection() {
        let languages = build_language_patterns();

        assert_eq!(
            get_language_for_file(std::path::Path::new("main.py"), &languages)
                .map(|i| languages[i].language_name),
            Some("python")
        );
        assert_eq!(
            get_language_for_file(std::path::Path::new("main.go"), &languages)
                .map(|i| languages[i].language_name),
            Some("go")
        );
        assert_eq!(
            get_language_for_file(std::path::Path::new("main.rs"), &languages)
                .map(|i| languages[i].language_name),
            Some("rust")
        );
        assert_eq!(
            get_language_for_file(std::path::Path::new("Dockerfile"), &languages)
                .map(|i| languages[i].language_name),
            Some("docker")
        );
        assert_eq!(
            get_language_for_file(std::path::Path::new("Makefile"), &languages)
                .map(|i| languages[i].language_name),
            Some("makefile")
        );
        assert_eq!(
            get_language_for_file(std::path::Path::new("test.rb"), &languages)
                .map(|i| languages[i].language_name),
            Some("ruby")
        );
        assert_eq!(
            get_language_for_file(std::path::Path::new("test.sh"), &languages)
                .map(|i| languages[i].language_name),
            Some("shell")
        );
    }
}
