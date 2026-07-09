use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

/// Schema rule for a single environment variable
#[derive(Debug, Clone)]
pub struct VarRule {
    pub name: String,
    pub required: bool,
    pub default: Option<String>,
    pub description: String,
    pub pattern: Option<Regex>,
    pub allowed_values: Option<Vec<String>>,
    pub secret: bool,
}

/// Validation error for a single variable
#[derive(Debug, Clone, Serialize)]
pub struct ValidationError {
    pub var_name: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Load rules from a schema .env.example file with comments
pub fn load_rules_from_example(path: &std::path::Path) -> anyhow::Result<Vec<VarRule>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_rules_from_content(&content))
}

/// Parse rules from content that follows .env.example conventions
/// Lines starting with # are descriptions, KEY=value defines the variable
pub fn parse_rules_from_content(content: &str) -> Vec<VarRule> {
    let mut rules = Vec::new();
    let mut current_description = String::new();
    let re_required = Regex::new(r"@required").unwrap();
    let re_secret = Regex::new(r"@secret").unwrap();
    let re_default = Regex::new(r"@default\s+(\S+)").unwrap();
    let re_pattern = Regex::new(r"@pattern\s+(.+)").unwrap();
    let re_allowed = Regex::new(r"@allowed\s+(.+)").unwrap();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let comment_text = trimmed.trim_start_matches('#').trim();
            if !current_description.is_empty() {
                current_description.push(' ');
            }
            current_description.push_str(comment_text);
        } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some(eq_pos) = trimmed.find('=') {
                let name = trimmed[..eq_pos].trim().to_string();
                let value_raw = trimmed[eq_pos + 1..].trim().to_string();

                let required = re_required.is_match(&current_description);
                let secret = re_secret.is_match(&current_description);
                let default = re_default
                    .captures(&current_description)
                    .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));
                let pattern = re_pattern
                    .captures(&current_description)
                    .and_then(|c| c.get(1).map(|m| Regex::new(m.as_str()).ok()))
                    .flatten();
                let allowed_values = re_allowed
                    .captures(&current_description)
                    .map(|c| c.get(1).unwrap().as_str().split(',').map(|s| s.trim().to_string()).collect());

                let description = current_description.trim().to_string();

                rules.push(VarRule {
                    name,
                    required,
                    default: if value_raw.is_empty() {
                        default.or(Some(value_raw))
                    } else {
                        Some(value_raw)
                    },
                    description,
                    pattern,
                    allowed_values,
                    secret,
                });
                current_description.clear();
            }
        }
    }

    rules
}

/// Validate a set of environment variables against rules
pub fn validate_vars(vars: &BTreeMap<String, String>, rules: &[VarRule]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let var_names: HashSet<&str> = vars.keys().map(|k| k.as_str()).collect();

    for rule in rules {
        if rule.required && !var_names.contains(rule.name.as_str()) {
            errors.push(ValidationError {
                var_name: rule.name.clone(),
                message: format!("Required variable '{}' is missing", rule.name),
                severity: Severity::Error,
            });
        }

        if let Some(value) = vars.get(&rule.name) {
            if value.is_empty() && rule.required {
                errors.push(ValidationError {
                    var_name: rule.name.clone(),
                    message: format!("Required variable '{}' is empty", rule.name),
                    severity: Severity::Error,
                });
            }

            if let Some(ref pattern) = rule.pattern {
                if !pattern.is_match(value) {
                    errors.push(ValidationError {
                        var_name: rule.name.clone(),
                        message: format!(
                            "'{}' does not match required pattern: {}",
                            rule.name,
                            pattern.as_str()
                        ),
                        severity: Severity::Error,
                    });
                }
            }

            if let Some(ref allowed) = rule.allowed_values {
                if !allowed.contains(value) {
                    errors.push(ValidationError {
                        var_name: rule.name.clone(),
                        message: format!(
                            "'{}' value '{}' is not in allowed values: {:?}",
                            rule.name, value, allowed
                        ),
                        severity: Severity::Error,
                    });
                }
            }
        }
    }

    // Check for unknown variables (present in env but not in rules)
    let rule_names: HashSet<&str> = rules.iter().map(|r| r.name.as_str()).collect();
    for name in var_names {
        if !rule_names.contains(name) {
            errors.push(ValidationError {
                var_name: name.to_string(),
                message: format!("Unexpected variable '{}' not defined in schema", name),
                severity: Severity::Warning,
            });
        }
    }

    errors
}

/// Suggest which vars to add to .env.example based on actual usage
pub fn suggest_vars_from_code(
    env_vars: &HashSet<String>,
    existing_rules: &[VarRule],
) -> Vec<String> {
    let known: HashSet<&str> = existing_rules.iter().map(|r| r.name.as_str()).collect();
    let mut suggestions: Vec<String> = env_vars
        .iter()
        .filter(|v| !known.contains(v.as_str()))
        .cloned()
        .collect();
    suggestions.sort();
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rules_from_example() {
        let content = r#"# Database connection URL @required
DATABASE_URL=
# API key for external service @required @secret
API_KEY=
# Log level @default info @allowed debug,info,warn,error
LOG_LEVEL=info
# Optional feature flag
FEATURE_X=false
"#;
        let rules = parse_rules_from_content(content);
        assert_eq!(rules.len(), 4);
        assert!(rules[0].required);
        assert!(!rules[0].secret);
        assert!(rules[1].required);
        assert!(rules[1].secret);
        assert!(rules[2].allowed_values.is_some());
        assert_eq!(rules[2].allowed_values.as_ref().unwrap().len(), 4);
        assert!(!rules[3].required);
    }

    #[test]
    fn test_validate_missing_required() {
        let mut vars = BTreeMap::new();
        vars.insert("LOG_LEVEL".to_string(), "info".to_string());

        let content = r#"# DB URL @required
DATABASE_URL=
# Log level @default info
LOG_LEVEL=info
"#;
        let rules = parse_rules_from_content(content);
        let errors = validate_vars(&vars, &rules);
        assert!(errors.iter().any(|e| e.var_name == "DATABASE_URL"));
    }

    #[test]
    fn test_validate_all_good() {
        let mut vars = BTreeMap::new();
        vars.insert("DATABASE_URL".to_string(), "postgres://localhost/db".to_string());
        vars.insert("LOG_LEVEL".to_string(), "debug".to_string());

        let content = r#"# DB URL @required
DATABASE_URL=
# Log level @default info @allowed debug,info,warn,error
LOG_LEVEL=info
"#;
        let rules = parse_rules_from_content(content);
        let errors = validate_vars(&vars, &rules);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_allowed_values() {
        let mut vars = BTreeMap::new();
        vars.insert("LOG_LEVEL".to_string(), "critical".to_string());

        let content = r#"# Log level @allowed debug,info,warn,error
LOG_LEVEL=info
"#;
        let rules = parse_rules_from_content(content);
        let errors = validate_vars(&vars, &rules);
        assert!(errors.iter().any(|e| e.var_name == "LOG_LEVEL"));
    }
}