use std::collections::BTreeMap;

/// Represents a parsed .env file entry
#[derive(Debug, Clone)]
pub struct EnvEntry {
    pub key: String,
    pub value: String,
    pub raw: String,
    pub line_number: usize,
    pub is_comment: bool,
    pub is_empty: bool,
}

/// Parse a .env file content into entries
pub fn parse_env(content: &str) -> Vec<EnvEntry> {
    let mut entries = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            entries.push(EnvEntry {
                key: String::new(),
                value: String::new(),
                raw: line.to_string(),
                line_number: i + 1,
                is_comment: false,
                is_empty: true,
            });
        } else if trimmed.starts_with('#') {
            entries.push(EnvEntry {
                key: String::new(),
                value: String::new(),
                raw: line.to_string(),
                line_number: i + 1,
                is_comment: true,
                is_empty: false,
            });
        } else {
            let (key, value) = parse_env_line(trimmed);
            entries.push(EnvEntry {
                key,
                value,
                raw: line.to_string(),
                line_number: i + 1,
                is_comment: false,
                is_empty: false,
            });
        }
    }
    entries
}

fn parse_env_line(line: &str) -> (String, String) {
    if let Some(eq_pos) = line.find('=') {
        let key = line[..eq_pos].trim().to_string();
        let value_raw = line[eq_pos + 1..].trim();
        let value = unescape_value(value_raw);
        (key, value)
    } else {
        (line.trim().to_string(), String::new())
    }
}

fn unescape_value(value: &str) -> String {
    let value = value
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(value);
    value.to_string()
}

/// Extract key-value pairs as a map
pub fn env_to_map(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for entry in parse_env(content) {
        if !entry.is_comment && !entry.is_empty && !entry.key.is_empty() {
            map.insert(entry.key, entry.value);
        }
    }
    map
}

/// Read a .env file
pub fn read_env_file(path: &std::path::Path) -> anyhow::Result<String> {
    Ok(std::fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let content = "DATABASE_URL=postgres://localhost:5432/mydb\nAPI_KEY=secret123\n";
        let entries = parse_env(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "DATABASE_URL");
        assert_eq!(entries[1].key, "API_KEY");
    }

    #[test]
    fn test_parse_with_comments() {
        let content = "# This is a comment\nKEY=value\n";
        let entries = parse_env(content);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_comment);
        assert!(!entries[1].is_comment);
    }

    #[test]
    fn test_parse_quoted_values() {
        let content = r#"NAME="John Doe""#;
        let entries = parse_env(content);
        assert_eq!(entries[0].value, "John Doe");
    }

    #[test]
    fn test_env_to_map() {
        let content = "A=1\nB=2\n# comment\nC=3\n";
        let map = env_to_map(content);
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("A").unwrap(), "1");
    }

    #[test]
    fn test_empty_lines() {
        let content = "A=1\n\nB=2\n";
        let entries = parse_env(content);
        assert_eq!(entries.len(), 3);
        assert!(entries[1].is_empty);
    }

    #[test]
    fn test_parse_value_with_special_chars() {
        let content = "URL=http://example.com:8080/path?q=1";
        let entries = parse_env(content);
        assert_eq!(entries[0].value, "http://example.com:8080/path?q=1");
    }
}
