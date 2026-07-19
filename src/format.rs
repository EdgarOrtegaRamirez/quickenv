use std::collections::BTreeMap;

/// Supported import/export formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormatType {
    Dotenv,
    DockerCompose,
    Kubernetes,
    CircleCI,
    GitHub,
    Json,
}

impl FormatType {
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "dotenv" | "env" => Ok(FormatType::Dotenv),
            "docker-compose" | "docker_compose" | "docker" => Ok(FormatType::DockerCompose),
            "kubernetes" | "k8s" | "secret" => Ok(FormatType::Kubernetes),
            "circleci" | "circle-ci" => Ok(FormatType::CircleCI),
            "github" | "github-actions" | "github_actions" | "gh" => Ok(FormatType::GitHub),
            "json" => Ok(FormatType::Json),
            _ => Err(anyhow::anyhow!("unsupported format: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FormatType::Dotenv => "dotenv",
            FormatType::DockerCompose => "docker-compose",
            FormatType::Kubernetes => "kubernetes",
            FormatType::CircleCI => "circleci",
            FormatType::GitHub => "github",
            FormatType::Json => "json",
        }
    }
}

/// Import env vars from a foreign format.
pub fn import_env(data: &str, format_type: FormatType) -> anyhow::Result<BTreeMap<String, String>> {
    match format_type {
        FormatType::Dotenv => import_dotenv(data),
        FormatType::DockerCompose => import_docker_compose(data),
        FormatType::Kubernetes => import_kubernetes(data),
        FormatType::CircleCI => import_circleci(data),
        FormatType::GitHub => import_github(data),
        FormatType::Json => import_json(data),
    }
}

/// Export env vars to a foreign format.
pub fn export_env(
    env: &BTreeMap<String, String>,
    format_type: FormatType,
) -> anyhow::Result<String> {
    match format_type {
        FormatType::Dotenv => export_dotenv(env),
        FormatType::DockerCompose => export_docker_compose(env),
        FormatType::Kubernetes => export_kubernetes(env),
        FormatType::CircleCI => export_circleci(env),
        FormatType::GitHub => export_github(env),
        FormatType::Json => export_json(env),
    }
}

// --- Dotenv ---

fn import_dotenv(data: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let mut value = line[eq_pos + 1..].trim().to_string();
            // Remove surrounding quotes
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }
            env.insert(key, value);
        }
    }
    Ok(env)
}

fn export_dotenv(env: &BTreeMap<String, String>) -> anyhow::Result<String> {
    let mut output = String::new();
    for (key, value) in env {
        if needs_quoting(value) {
            output.push_str(&format!("{}={:?}\n", key, value));
        } else {
            output.push_str(&format!("{}={}\n", key, value));
        }
    }
    Ok(output)
}

// --- Docker Compose ---

fn import_docker_compose(data: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for line in data.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("- ") {
            if let Some(eq_pos) = stripped.find('=') {
                let key = stripped[..eq_pos].trim().to_string();
                let value = stripped[eq_pos + 1..].trim().to_string();
                let value = value.trim_matches('"').to_string();
                env.insert(key, value);
            }
        }
    }
    Ok(env)
}

fn export_docker_compose(env: &BTreeMap<String, String>) -> anyhow::Result<String> {
    let mut output = String::new();
    output.push_str("version: '3.8'\n");
    output.push_str("services:\n  app:\n    environment:\n");
    for (key, value) in env {
        output.push_str(&format!("      - {}={}\n", key, value));
    }
    Ok(output)
}

// --- Kubernetes ---

fn import_kubernetes(data: &str) -> anyhow::Result<BTreeMap<String, String>> {
    // Try to parse as JSON
    let config: Result<serde_json::Value, _> = serde_json::from_str(data);
    match config {
        Ok(serde_json::Value::Object(map)) => {
            let mut env = BTreeMap::new();
            if let Some(data_obj) = map.get("data").and_then(|v| v.as_object()) {
                for (key, val) in data_obj {
                    if let Some(val_str) = val.as_str() {
                        env.insert(key.clone(), val_str.to_string());
                    }
                }
            }
            Ok(env)
        }
        _ => {
            // Fallback: try to parse as YAML-like env vars
            import_dotenv(data)
        }
    }
}

fn export_kubernetes(env: &BTreeMap<String, String>) -> anyhow::Result<String> {
    let data: serde_json::Value = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": {
            "name": "app-env"
        },
        "type": "Opaque",
        "data": env
    });
    Ok(serde_json::to_string_pretty(&data)?)
}

// --- CircleCI ---

fn import_circleci(data: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for line in data.lines() {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line);
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let mut value = line[eq_pos + 1..].trim().to_string();
            value = value.trim_matches('"').trim_matches('\'').to_string();
            env.insert(key, value);
        }
    }
    Ok(env)
}

fn export_circleci(env: &BTreeMap<String, String>) -> anyhow::Result<String> {
    let mut output = String::new();
    output.push_str("# CircleCI environment variables\n");
    for (key, value) in env {
        output.push_str(&format!("export {}={:?}\n", key, value));
    }
    Ok(output)
}

// --- GitHub Actions ---

fn import_github(data: &str) -> anyhow::Result<BTreeMap<String, String>> {
    import_dotenv(data)
}

fn export_github(env: &BTreeMap<String, String>) -> anyhow::Result<String> {
    let mut output = String::new();
    output.push_str("# GitHub Actions environment variables\n");
    for (key, value) in env {
        output.push_str(&format!("{}={}\n", key, value));
    }
    Ok(output)
}

// --- JSON ---

fn import_json(data: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let config: serde_json::Value = serde_json::from_str(data)?;
    let mut env = BTreeMap::new();
    if let serde_json::Value::Object(map) = config {
        for (key, val) in map {
            match val {
                serde_json::Value::String(s) => {
                    env.insert(key, s);
                }
                other => {
                    env.insert(key, format!("{}", other));
                }
            }
        }
    }
    Ok(env)
}

fn export_json(env: &BTreeMap<String, String>) -> anyhow::Result<String> {
    let value: serde_json::Value = serde_json::json!(env);
    Ok(serde_json::to_string_pretty(&value)?)
}

fn needs_quoting(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    value.contains(' ')
        || value.contains('#')
        || value.contains('=')
        || value.contains('"')
        || value.contains('\'')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), v.to_string());
        }
        m
    }

    #[test]
    fn test_import_dotenv_simple() {
        let data = "KEY=value\nFOO=bar\n";
        let env = import_dotenv(data).unwrap();
        assert_eq!(env.len(), 2);
        assert_eq!(env.get("KEY").unwrap(), "value");
    }

    #[test]
    fn test_import_dotenv_quoted() {
        let data = "NAME=\"John Doe\"\n";
        let env = import_dotenv(data).unwrap();
        assert_eq!(env.get("NAME").unwrap(), "John Doe");
    }

    #[test]
    fn test_export_dotenv() {
        let env = make_env(&[("KEY", "value"), ("FOO", "bar")]);
        let out = export_dotenv(&env).unwrap();
        assert!(out.contains("KEY=value"));
        assert!(out.contains("FOO=bar"));
    }

    #[test]
    fn test_import_json() {
        let data = r#"{"DB_HOST": "localhost", "DB_PORT": "5432"}"#;
        let env = import_json(data).unwrap();
        assert_eq!(env.len(), 2);
        assert_eq!(env.get("DB_HOST").unwrap(), "localhost");
    }

    #[test]
    fn test_export_json() {
        let env = make_env(&[("KEY", "value")]);
        let out = export_json(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["KEY"], "value");
    }

    #[test]
    fn test_export_docker_compose() {
        let env = make_env(&[("DB_HOST", "localhost")]);
        let out = export_docker_compose(&env).unwrap();
        assert!(out.contains("version: '3.8'"));
        assert!(out.contains("DB_HOST=localhost"));
    }

    #[test]
    fn test_export_kubernetes() {
        let env = make_env(&[("DB_HOST", "localhost")]);
        let out = export_kubernetes(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["kind"], "Secret");
        assert_eq!(parsed["data"]["DB_HOST"], "localhost");
    }

    #[test]
    fn test_export_circleci() {
        let env = make_env(&[("DB_HOST", "localhost")]);
        let out = export_circleci(&env).unwrap();
        assert!(out.contains("export DB_HOST="));
    }

    #[test]
    fn test_export_github() {
        let env = make_env(&[("DB_HOST", "localhost")]);
        let out = export_github(&env).unwrap();
        assert!(out.contains("DB_HOST=localhost"));
    }

    #[test]
    fn test_import_circleci() {
        let data = "export DB_HOST=localhost\nexport DB_PORT=5432\n";
        let env = import_circleci(data).unwrap();
        assert_eq!(env.get("DB_HOST").unwrap(), "localhost");
    }

    #[test]
    fn test_format_type_parse() {
        assert_eq!(FormatType::from_str("json").unwrap(), FormatType::Json);
        assert_eq!(
            FormatType::from_str("docker-compose").unwrap(),
            FormatType::DockerCompose
        );
        assert_eq!(
            FormatType::from_str("kubernetes").unwrap(),
            FormatType::Kubernetes
        );
        assert_eq!(
            FormatType::from_str("circleci").unwrap(),
            FormatType::CircleCI
        );
        assert_eq!(FormatType::from_str("github").unwrap(), FormatType::GitHub);
        assert_eq!(FormatType::from_str("dotenv").unwrap(), FormatType::Dotenv);
    }

    #[test]
    fn test_import_docker_compose() {
        let data = "services:\n  app:\n    environment:\n      - DB_HOST=localhost\n      - DB_PORT=5432\n";
        let env = import_docker_compose(data).unwrap();
        assert_eq!(env.get("DB_HOST").unwrap(), "localhost");
    }

    #[test]
    fn test_import_kubernetes() {
        let data = r#"{"apiVersion": "v1", "kind": "Secret", "data": {"DB_HOST": "bG9jYWxob3N0"}}"#;
        let env = import_kubernetes(data).unwrap();
        assert_eq!(env.get("DB_HOST").unwrap(), "bG9jYWxob3N0");
    }
}
