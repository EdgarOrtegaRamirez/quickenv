use std::collections::BTreeMap;

/// Represents a migration operation.
#[derive(Debug, Clone)]
pub struct MigrationOp {
    pub action: String, // "add", "remove", "rename", "set"
    pub key: String,
    pub new_key: Option<String>,
    pub value: String,
}

/// Represents the result of a migration.
#[derive(Debug, Clone)]
pub struct MigrateResult {
    pub applied: Vec<MigrationOp>,
    pub errors: Vec<String>,
    pub env_map: BTreeMap<String, String>,
}

/// Apply migration operations to an environment variable map.
pub fn migrate(env_map: &BTreeMap<String, String>, ops: &[MigrationOp]) -> MigrateResult {
    let mut result_map = env_map.clone();
    let mut applied = Vec::new();
    let mut errors = Vec::new();

    for op in ops {
        match op.action.as_str() {
            "add" => {
                if result_map.contains_key(&op.key) {
                    errors.push(format!("add: key \"{}\" already exists", op.key));
                } else {
                    result_map.insert(op.key.clone(), op.value.clone());
                    applied.push(op.clone());
                }
            }
            "remove" => {
                if !result_map.contains_key(&op.key) {
                    errors.push(format!("remove: key \"{}\" not found", op.key));
                } else {
                    result_map.remove(&op.key);
                    applied.push(op.clone());
                }
            }
            "rename" => {
                let val = match result_map.remove(&op.key) {
                    Some(v) => v,
                    None => {
                        errors.push(format!("rename: key \"{}\" not found", op.key));
                        continue;
                    }
                };
                let new_key = op.new_key.as_deref().unwrap_or("");
                if new_key.is_empty() {
                    errors.push("rename: new_key is required".to_string());
                    result_map.insert(op.key.clone(), val); // put back
                    continue;
                }
                if result_map.contains_key(new_key) {
                    errors.push(format!("rename: target key \"{}\" already exists", new_key));
                    result_map.insert(op.key.clone(), val); // put back
                    continue;
                }
                result_map.insert(new_key.to_string(), val);
                applied.push(op.clone());
            }
            "set" => {
                result_map.insert(op.key.clone(), op.value.clone());
                applied.push(op.clone());
            }
            _ => {
                errors.push(format!("unknown action: \"{}\"", op.action));
            }
        }
    }

    MigrateResult {
        applied,
        errors,
        env_map: result_map,
    }
}

/// Parse migration operations from a string.
/// Format: "action:key[:value]" one per line. Lines starting with # are comments.
/// For rename: "rename:old_key:->new_key" or "rename:old_key:new_key"
pub fn parse_migration_ops(data: &str) -> anyhow::Result<Vec<MigrationOp>> {
    let mut ops = Vec::new();

    for (i, line) in data.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!(
                "line {}: invalid format \"{}\" (expected action:key[:value])",
                i + 1,
                line
            ));
        }

        let action = parts[0].trim().to_string();
        let key = parts[1].trim().to_string();

        let mut op = MigrationOp {
            action,
            key,
            new_key: None,
            value: String::new(),
        };

        if parts.len() > 2 {
            let val = parts[2].trim().to_string();
            if op.action == "rename" {
                // Handle "rename:old_key:->new_key" format
                if let Some(pos) = val.find("->") {
                    op.new_key = Some(val[pos + 2..].trim().to_string());
                } else {
                    op.new_key = Some(val);
                }
            } else {
                op.value = val;
            }
        }

        ops.push(op);
    }

    Ok(ops)
}

/// Format a migration result for display.
pub fn format_migrate_result(result: &MigrateResult) -> String {
    let mut output = String::new();

    if !result.applied.is_empty() {
        output.push_str("Applied operations:\n");
        for op in &result.applied {
            match op.action.as_str() {
                "add" => output.push_str(&format!("  + {}={}\n", op.key, op.value)),
                "remove" => output.push_str(&format!("  - {}\n", op.key)),
                "rename" => {
                    let new_key = op.new_key.as_deref().unwrap_or("");
                    output.push_str(&format!("  ~ {} → {}\n", op.key, new_key));
                }
                "set" => output.push_str(&format!("  = {}={}\n", op.key, op.value)),
                _ => {}
            }
        }
    }

    if !result.errors.is_empty() {
        output.push_str("\nErrors:\n");
        for err in &result.errors {
            output.push_str(&format!("  ✗ {}\n", err));
        }
    }

    output.push_str(&format!(
        "\n{} operations applied, {} errors\n",
        result.applied.len(),
        result.errors.len()
    ));

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_add() {
        let mut env = BTreeMap::new();
        env.insert("EXISTING".to_string(), "value".to_string());

        let ops = vec![MigrationOp {
            action: "add".to_string(),
            key: "NEW_KEY".to_string(),
            new_key: None,
            value: "new_value".to_string(),
        }];

        let result = migrate(&env, &ops);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.env_map.get("NEW_KEY").unwrap(), "new_value");
    }

    #[test]
    fn test_migrate_add_duplicate() {
        let mut env = BTreeMap::new();
        env.insert("KEY".to_string(), "value".to_string());

        let ops = vec![MigrationOp {
            action: "add".to_string(),
            key: "KEY".to_string(),
            new_key: None,
            value: "new".to_string(),
        }];

        let result = migrate(&env, &ops);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("already exists"));
    }

    #[test]
    fn test_migrate_remove() {
        let mut env = BTreeMap::new();
        env.insert("KEY".to_string(), "value".to_string());

        let ops = vec![MigrationOp {
            action: "remove".to_string(),
            key: "KEY".to_string(),
            new_key: None,
            value: String::new(),
        }];

        let result = migrate(&env, &ops);
        assert_eq!(result.errors.len(), 0);
        assert!(!result.env_map.contains_key("KEY"));
    }

    #[test]
    fn test_migrate_rename() {
        let mut env = BTreeMap::new();
        env.insert("OLD".to_string(), "value".to_string());

        let ops = vec![MigrationOp {
            action: "rename".to_string(),
            key: "OLD".to_string(),
            new_key: Some("NEW".to_string()),
            value: String::new(),
        }];

        let result = migrate(&env, &ops);
        assert_eq!(result.errors.len(), 0);
        assert!(!result.env_map.contains_key("OLD"));
        assert_eq!(result.env_map.get("NEW").unwrap(), "value");
    }

    #[test]
    fn test_migrate_set() {
        let mut env = BTreeMap::new();
        env.insert("KEY".to_string(), "old".to_string());

        let ops = vec![MigrationOp {
            action: "set".to_string(),
            key: "KEY".to_string(),
            new_key: None,
            value: "new_value".to_string(),
        }];

        let result = migrate(&env, &ops);
        assert_eq!(result.errors.len(), 0);
        assert_eq!(result.env_map.get("KEY").unwrap(), "new_value");
    }

    #[test]
    fn test_parse_ops_simple() {
        let data = "add:DB_HOST:localhost\nadd:DB_PORT:5432\n";
        let ops = parse_migration_ops(data).unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].action, "add");
        assert_eq!(ops[0].key, "DB_HOST");
        assert_eq!(ops[0].value, "localhost");
    }

    #[test]
    fn test_parse_ops_rename() {
        let data = "rename:OLD_KEY:->NEW_KEY\n";
        let ops = parse_migration_ops(data).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].action, "rename");
        assert_eq!(ops[0].key, "OLD_KEY");
        assert_eq!(ops[0].new_key.as_deref().unwrap(), "NEW_KEY");
    }

    #[test]
    fn test_parse_ops_skip_comments() {
        let data = "# comment\naction:key:value\n";
        let ops = parse_migration_ops(data).unwrap();
        assert_eq!(ops.len(), 1);
    }
}