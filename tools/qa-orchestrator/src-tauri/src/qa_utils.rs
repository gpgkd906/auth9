use anyhow::Result;
use std::collections::HashSet;
use std::path::{Component, Path};

pub fn validate_workspace_rel_path(raw: &str, field: &str) -> Result<()> {
    let path = raw.trim();
    if path.is_empty() {
        anyhow::bail!("{} cannot be empty", field);
    }

    let parsed = Path::new(path);
    if parsed.is_absolute() {
        anyhow::bail!("{} must be a relative path: {}", field, raw);
    }

    if parsed
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        anyhow::bail!("{} cannot include '..': {}", field, raw);
    }

    Ok(())
}

pub fn new_ticket_diff(before: &[String], after: &[String]) -> Vec<String> {
    let before_set: HashSet<&String> = before.iter().collect();
    after
        .iter()
        .filter(|path| !before_set.contains(path))
        .cloned()
        .collect()
}

pub fn render_template(template: &str, rel_path: &str, ticket_paths: &[String]) -> String {
    template
        .replace("{rel_path}", rel_path)
        .replace("{ticket_paths}", &ticket_paths.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_workspace_rel_path_accepts_normal_relative_paths() {
        assert!(validate_workspace_rel_path("docs/qa", "field").is_ok());
        assert!(validate_workspace_rel_path("config/default.yaml", "field").is_ok());
        assert!(validate_workspace_rel_path("a-b_c/1", "field").is_ok());
    }

    #[test]
    fn validate_workspace_rel_path_rejects_empty_input() {
        assert!(validate_workspace_rel_path("", "f").is_err());
        assert!(validate_workspace_rel_path("   ", "f").is_err());
    }

    #[test]
    fn validate_workspace_rel_path_rejects_absolute_path() {
        assert!(validate_workspace_rel_path("/tmp/data", "f").is_err());
    }

    #[test]
    fn validate_workspace_rel_path_rejects_parent_segments() {
        assert!(validate_workspace_rel_path("../docs", "f").is_err());
        assert!(validate_workspace_rel_path("docs/../../x", "f").is_err());
    }

    #[test]
    fn render_template_replaces_placeholders() {
        let template = "run {rel_path} --tickets {ticket_paths}";
        let tickets = vec!["a.md".to_string(), "b.md".to_string()];
        let rendered = render_template(template, "docs/qa/1.md", &tickets);
        assert_eq!(rendered, "run docs/qa/1.md --tickets a.md b.md");
    }

    #[test]
    fn render_template_handles_empty_ticket_paths() {
        let rendered = render_template("{rel_path}:{ticket_paths}", "x.md", &[]);
        assert_eq!(rendered, "x.md:");
    }

    #[test]
    fn new_ticket_diff_returns_only_new_items_with_original_order() {
        let before = vec!["a".to_string(), "b".to_string()];
        let after = vec!["b".to_string(), "c".to_string(), "d".to_string()];
        let diff = new_ticket_diff(&before, &after);
        assert_eq!(diff, vec!["c".to_string(), "d".to_string()]);
    }

    #[test]
    fn new_ticket_diff_returns_empty_when_no_new_items() {
        let before = vec!["a".to_string(), "b".to_string()];
        let after = vec!["a".to_string(), "b".to_string()];
        let diff = new_ticket_diff(&before, &after);
        assert!(diff.is_empty());
    }

    #[test]
    fn new_ticket_diff_keeps_duplicates_if_after_has_duplicates() {
        let before = vec!["a".to_string()];
        let after = vec!["b".to_string(), "b".to_string()];
        let diff = new_ticket_diff(&before, &after);
        assert_eq!(diff, vec!["b".to_string(), "b".to_string()]);
    }
}
