use std::collections::BTreeMap;

/// Sync strategies for env file synchronization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStrategy {
    Overwrite,
    Merge,
    Skip,
    AddOnly,
    UpdateOnly,
}

impl SyncStrategy {
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "overwrite" => Ok(SyncStrategy::Overwrite),
            "merge" => Ok(SyncStrategy::Merge),
            "skip" => Ok(SyncStrategy::Skip),
            "add-only" | "addonly" | "add" => Ok(SyncStrategy::AddOnly),
            "update-only" | "updateonly" | "update" => Ok(SyncStrategy::UpdateOnly),
            _ => Err(anyhow::anyhow!("unknown sync strategy: {}", s)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SyncStrategy::Overwrite => "overwrite",
            SyncStrategy::Merge => "merge",
            SyncStrategy::Skip => "skip",
            SyncStrategy::AddOnly => "add-only",
            SyncStrategy::UpdateOnly => "update-only",
        }
    }
}

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    pub unchanged: Vec<String>,
    pub skipped: Vec<String>,
}

impl SyncResult {
    pub fn summary(&self) -> String {
        format!(
            "added={} updated={} removed={} unchanged={} skipped={}",
            self.added.len(),
            self.updated.len(),
            self.removed.len(),
            self.unchanged.len(),
            self.skipped.len(),
        )
    }
}

/// Synchronize target env vars based on source and strategy.
pub fn sync(
    source: &BTreeMap<String, String>,
    target: &BTreeMap<String, String>,
    strategy: SyncStrategy,
) -> (BTreeMap<String, String>, SyncResult) {
    let mut result = SyncResult {
        added: Vec::new(),
        updated: Vec::new(),
        removed: Vec::new(),
        unchanged: Vec::new(),
        skipped: Vec::new(),
    };

    let mut output = target.clone();

    for (key, src_val) in source {
        if let Some(tgt_val) = output.get(key) {
            // Key exists in target
            match strategy {
                SyncStrategy::Overwrite | SyncStrategy::Merge | SyncStrategy::UpdateOnly => {
                    if tgt_val != src_val {
                        output.insert(key.clone(), src_val.clone());
                        result.updated.push(key.clone());
                    } else {
                        result.unchanged.push(key.clone());
                    }
                }
                SyncStrategy::Skip | SyncStrategy::AddOnly => {
                    result.skipped.push(key.clone());
                }
            }
        } else {
            // Key doesn't exist in target
            match strategy {
                SyncStrategy::Overwrite | SyncStrategy::Merge | SyncStrategy::AddOnly => {
                    output.insert(key.clone(), src_val.clone());
                    result.added.push(key.clone());
                }
                SyncStrategy::Skip | SyncStrategy::UpdateOnly => {
                    result.skipped.push(key.clone());
                }
            }
        }
    }

    // For overwrite strategy, remove keys in target not in source
    if strategy == SyncStrategy::Overwrite {
        let source_keys: std::collections::HashSet<String> = source.keys().cloned().collect();
        let target_keys: Vec<String> = target.keys().cloned().collect();
        for key in &target_keys {
            if !source_keys.contains(key) {
                output.remove(key);
                result.removed.push(key.clone());
            }
        }
    }

    (output, result)
}

/// Format a sync result for display.
pub fn format_sync_result(result: &SyncResult) -> String {
    let mut output = String::new();

    if !result.added.is_empty() {
        output.push_str("Added:\n");
        for k in &result.added {
            output.push_str(&format!("  + {}\n", k));
        }
    }

    if !result.updated.is_empty() {
        output.push_str("Updated:\n");
        for k in &result.updated {
            output.push_str(&format!("  ~ {}\n", k));
        }
    }

    if !result.removed.is_empty() {
        output.push_str("Removed:\n");
        for k in &result.removed {
            output.push_str(&format!("  - {}\n", k));
        }
    }

    if !result.skipped.is_empty() {
        output.push_str("Skipped:\n");
        for k in &result.skipped {
            output.push_str(&format!("  = {}\n", k));
        }
    }

    if !result.unchanged.is_empty() {
        output.push_str(&format!(
            "Unchanged: {} variables\n",
            result.unchanged.len()
        ));
    }

    output.push_str(&format!("\n{}\n", result.summary()));
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), v.to_string());
        }
        m
    }

    #[test]
    fn test_sync_overwrite_adds_new() {
        let source = make_map(&[("A", "1"), ("B", "2")]);
        let target = make_map(&[("A", "old")]);
        let (output, result) = sync(&source, &target, SyncStrategy::Overwrite);
        assert_eq!(output.get("A").unwrap(), "1");
        assert_eq!(output.get("B").unwrap(), "2");
        assert!(result.added.contains(&"B".to_string()));
    }

    #[test]
    fn test_sync_overwrite_removes_extra() {
        let source = make_map(&[("A", "1")]);
        let target = make_map(&[("A", "old"), ("B", "extra")]);
        let (output, _) = sync(&source, &target, SyncStrategy::Overwrite);
        assert!(!output.contains_key("B"));
    }

    #[test]
    fn test_sync_merge_keeps_extra() {
        let source = make_map(&[("A", "1")]);
        let target = make_map(&[("A", "old"), ("B", "extra")]);
        let (output, _) = sync(&source, &target, SyncStrategy::Merge);
        assert_eq!(output.get("A").unwrap(), "1");
        assert_eq!(output.get("B").unwrap(), "extra");
    }

    #[test]
    fn test_sync_skip_does_nothing() {
        let source = make_map(&[("A", "new")]);
        let target = make_map(&[("A", "old")]);
        let (output, result) = sync(&source, &target, SyncStrategy::Skip);
        assert_eq!(output.get("A").unwrap(), "old");
        assert!(result.skipped.contains(&"A".to_string()));
    }

    #[test]
    fn test_sync_add_only() {
        let source = make_map(&[("A", "new"), ("B", "new")]);
        let target = make_map(&[("A", "old")]);
        let (output, result) = sync(&source, &target, SyncStrategy::AddOnly);
        assert_eq!(output.get("A").unwrap(), "old"); // unchanged
        assert_eq!(output.get("B").unwrap(), "new"); // added
        assert!(result.added.contains(&"B".to_string()));
        assert!(result.skipped.contains(&"A".to_string()));
    }

    #[test]
    fn test_sync_update_only() {
        let source = make_map(&[("A", "new"), ("B", "brand_new")]);
        let target = make_map(&[("A", "old"), ("C", "existing")]);
        let (output, result) = sync(&source, &target, SyncStrategy::UpdateOnly);
        assert_eq!(output.get("A").unwrap(), "new"); // updated
        assert!(!output.contains_key("B")); // not added (new key)
        assert!(result.updated.contains(&"A".to_string()));
        assert!(result.skipped.contains(&"B".to_string()));
    }

    #[test]
    fn test_sync_strategy_parse() {
        assert_eq!(
            SyncStrategy::from_str("overwrite").unwrap(),
            SyncStrategy::Overwrite
        );
        assert_eq!(
            SyncStrategy::from_str("merge").unwrap(),
            SyncStrategy::Merge
        );
        assert_eq!(SyncStrategy::from_str("skip").unwrap(), SyncStrategy::Skip);
        assert_eq!(
            SyncStrategy::from_str("add-only").unwrap(),
            SyncStrategy::AddOnly
        );
        assert_eq!(
            SyncStrategy::from_str("update-only").unwrap(),
            SyncStrategy::UpdateOnly
        );
    }

    #[test]
    fn test_sync_format_result() {
        let result = SyncResult {
            added: vec!["A".to_string()],
            updated: vec![],
            removed: vec![],
            unchanged: vec!["B".to_string()],
            skipped: vec![],
        };
        let formatted = format_sync_result(&result);
        assert!(formatted.contains("Added:"));
        assert!(formatted.contains("+ A"));
        assert!(formatted.contains("Unchanged: 1 variables"));
        assert!(formatted.contains("added=1"));
    }
}
