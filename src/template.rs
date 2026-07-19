use std::collections::BTreeMap;

use crate::envfile;

/// Represents a template variable extracted from an env file.
#[derive(Debug, Clone)]
pub struct TemplateVar {
    pub key: String,
    pub value: String,
}

/// Represents an .env template with variables for substitution.
#[derive(Debug, Clone)]
pub struct Template {
    pub vars: Vec<TemplateVar>,
    pub env_content: String,
}

/// Parse a template string, extracting ${VAR} and ${VAR:-default} placeholders.
pub fn parse_template(data: &str) -> Template {
    let mut vars: Vec<TemplateVar> = Vec::new();
    let mut seen_keys = std::collections::HashSet::new();

    let re = regex::Regex::new(r#"\$\{(\w+)(?::-([^}]*))?\}"#).unwrap();
    for cap in re.captures_iter(data) {
        let key = cap[1].to_string();
        let default = cap
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        if seen_keys.insert(key.clone()) {
            vars.push(TemplateVar {
                key,
                value: default,
            });
        }
    }

    vars.sort_by(|a, b| a.key.cmp(&b.key));

    Template {
        vars,
        env_content: data.to_string(),
    }
}

/// Load a template from a file.
pub fn load_template(path: &std::path::Path) -> anyhow::Result<Template> {
    let data = std::fs::read_to_string(path)?;
    Ok(parse_template(&data))
}

/// Render a template with the given variable values.
pub fn render_template(template: &Template, values: &BTreeMap<String, String>) -> String {
    let mut result = template.env_content.clone();
    for (key, value) in values {
        // Replace ${KEY} and ${KEY:-default} forms
        let re = regex::Regex::new(&format!(r#"\$\{{{}(?::-([^}}]*))?\}}"#, regex::escape(key)))
            .unwrap();
        result = re.replace_all(&result, value.as_str()).to_string();
    }
    result
}

/// Generate a template from an existing env file content.
/// Converts values to ${KEY} placeholders.
pub fn generate_from_env(content: &str) -> Template {
    let entries = envfile::parse_env(content);
    let mut vars = Vec::new();
    let mut output_lines = Vec::new();

    for entry in &entries {
        if entry.is_comment || entry.is_empty || entry.key.is_empty() {
            output_lines.push(entry.raw.clone());
        } else {
            vars.push(TemplateVar {
                key: entry.key.clone(),
                value: String::new(),
            });
            output_lines.push(format!("{}=${{{}}}", entry.key, entry.key));
        }
    }

    vars.sort_by(|a, b| a.key.cmp(&b.key));

    Template {
        vars,
        env_content: output_lines.join("\n"),
    }
}

/// Format a template for display, listing required and optional vars.
pub fn format_template(template: &Template, _show_defaults: bool) -> String {
    let mut output = String::new();
    output.push_str("# Environment Template\n");
    output.push_str(&format!("# Variables: {}\n\n", template.vars.len()));

    if !template.vars.is_empty() {
        let required: Vec<&TemplateVar> = template
            .vars
            .iter()
            .filter(|v| v.value.is_empty())
            .collect();
        let optional: Vec<&TemplateVar> = template
            .vars
            .iter()
            .filter(|v| !v.value.is_empty())
            .collect();

        if !required.is_empty() {
            output.push_str("# Required variables:\n");
            for v in &required {
                output.push_str(&format!("#   {}\n", v.key));
            }
        }
        if !optional.is_empty() {
            output.push_str("#\n# Optional variables (with defaults):\n");
            for v in &optional {
                output.push_str(&format!("#   {}={}\n", v.key, v.value));
            }
        }
        if !required.is_empty() || !optional.is_empty() {
            output.push('\n');
        }
    }

    output.push_str(&template.env_content);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_template_simple() {
        let data = "DB_URL=${DB_URL}\nAPI_KEY=${API_KEY}\n";
        let tmpl = parse_template(data);
        assert_eq!(tmpl.vars.len(), 2);
        assert_eq!(tmpl.vars[0].key, "API_KEY");
        assert_eq!(tmpl.vars[1].key, "DB_URL");
    }

    #[test]
    fn test_parse_template_with_defaults() {
        let data = "PORT=${PORT:-3000}\nHOST=${HOST:-localhost}";
        let tmpl = parse_template(data);
        assert_eq!(tmpl.vars.len(), 2);
        assert_eq!(tmpl.vars[0].key, "HOST"); // sorted alphabetically
        assert_eq!(tmpl.vars[0].value, "localhost");
        assert_eq!(tmpl.vars[1].key, "PORT");
        assert_eq!(tmpl.vars[1].value, "3000");
    }

    #[test]
    fn test_parse_template_dedup() {
        let data = "DB_URL=${DB_URL}\nDB_URL_FALLBACK=${DB_URL}";
        let tmpl = parse_template(data);
        assert_eq!(tmpl.vars.len(), 1);
        assert_eq!(tmpl.vars[0].key, "DB_URL");
    }

    #[test]
    fn test_render_template() {
        let data = "HOST=${HOST}\nPORT=${PORT:-8080}";
        let tmpl = parse_template(data);
        let mut values = BTreeMap::new();
        values.insert("HOST".to_string(), "example.com".to_string());
        values.insert("PORT".to_string(), "9090".to_string());
        let rendered = render_template(&tmpl, &values);
        assert!(rendered.contains("HOST=example.com"));
        assert!(rendered.contains("PORT=9090"));
    }

    #[test]
    fn test_render_missing_var() {
        let data = "KEY=${KEY}";
        let tmpl = parse_template(data);
        let values = BTreeMap::new();
        let rendered = render_template(&tmpl, &values);
        assert_eq!(rendered, data);
    }

    #[test]
    fn test_generate_from_env() {
        let content = "DB_URL=postgres://localhost/mydb\n# comment\nAPI_KEY=secret\n";
        let tmpl = generate_from_env(content);
        assert_eq!(tmpl.vars.len(), 2);
        assert!(tmpl.env_content.contains("DB_URL=${DB_URL}"));
        assert!(tmpl.env_content.contains("API_KEY=${API_KEY}"));
        assert!(tmpl.env_content.contains("# comment"));
    }

    #[test]
    fn test_format_template_required_only() {
        let data = "KEY=${KEY}\n";
        let tmpl = parse_template(data);
        let formatted = format_template(&tmpl, false);
        assert!(formatted.contains("# Required variables:"));
        assert!(formatted.contains("KEY=${KEY}"));
    }
}
