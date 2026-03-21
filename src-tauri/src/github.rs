use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A parsed action item from the meeting notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub title: String,
    pub assignee: Option<String>,
    pub body: String,
}

/// A successfully created GitHub issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedIssue {
    pub number: u64,
    pub title: String,
    pub url: String,
}

/// Parse action items from Claude's meeting notes markdown.
///
/// Expects lines formatted as:
///   - [ ] **Assignee**: Task description
///   - [ ] Task description
///   - [ ] @assignee Task description
pub fn parse_action_items(notes: &str) -> Vec<ActionItem> {
    let mut items = Vec::new();

    for line in notes.lines() {
        let trimmed = line.trim();

        // Match checkbox lines anywhere in the document: - [ ], - [x], * [ ]
        // Checkboxes only appear in the Action Items section of the template,
        // so no section-gating is needed — and avoids language-specific header matching.
        let checkbox_content = if trimmed.starts_with("- [ ] ") {
            Some(&trimmed[6..])
        } else if trimmed.starts_with("- [x] ") {
            Some(&trimmed[6..])
        } else if trimmed.starts_with("* [ ] ") {
            Some(&trimmed[6..])
        } else {
            None
        };

        if let Some(content) = checkbox_content {
            let content = content.trim();
            if content.is_empty() {
                continue;
            }

            let (assignee, title) = parse_assignee_and_title(content);

            items.push(ActionItem {
                title: title.clone(),
                assignee,
                body: format!("Action item from meeting notes.\n\n> {}", content),
            });
        }
    }

    items
}

/// Parse assignee from action item text.
///
/// Supported formats:
///   **David**: Do something → assignee="David", title="Do something"
///   @david Do something    → assignee="david", title="Do something"
///   (David) Do something   → assignee="David", title="Do something"
///   David : Do something   → assignee="David", title="Do something"
///   Just a task            → assignee=None, title="Just a task"
fn parse_assignee_and_title(content: &str) -> (Option<String>, String) {
    // Pattern 1: **Name**: task
    if content.starts_with("**") {
        if let Some(end) = content.find("**:") {
            let name = content[2..end].trim().to_string();
            let task = content[end + 3..].trim().to_string();
            if !task.is_empty() {
                return (Some(name), task);
            }
        }
        // Also handle **Name** : task (with space before colon)
        if let Some(end) = content[2..].find("**") {
            let name = content[2..2 + end].trim().to_string();
            let rest = content[2 + end + 2..].trim();
            let task = rest.strip_prefix(':').unwrap_or(rest).trim().to_string();
            if !task.is_empty() {
                return (Some(name), task);
            }
        }
    }

    // Pattern 2: @name task
    if content.starts_with('@') {
        if let Some(space) = content.find(' ') {
            let name = content[1..space].to_string();
            let task = content[space + 1..].trim().to_string();
            return (Some(name), task);
        }
    }

    // Pattern 3: (Name) task
    if content.starts_with('(') {
        if let Some(end) = content.find(')') {
            let name = content[1..end].trim().to_string();
            let task = content[end + 1..].trim().to_string();
            if !task.is_empty() {
                return (Some(name), task);
            }
        }
    }

    // Pattern 4: Name : task or Name: task (only if name is ≤ 3 words)
    if let Some(colon_pos) = content.find(':') {
        let potential_name = content[..colon_pos].trim();
        let word_count = potential_name.split_whitespace().count();
        if word_count <= 3
            && !potential_name.is_empty()
            && potential_name.chars().next().map_or(false, |c| c.is_uppercase())
        {
            let name = potential_name
                .trim_start_matches("**")
                .trim_end_matches("**")
                .trim()
                .to_string();
            let task = content[colon_pos + 1..].trim().to_string();
            if !task.is_empty() {
                return (Some(name), task);
            }
        }
    }

    // No assignee detected
    (None, content.to_string())
}

/// Create GitHub issues from action items using `gh` CLI.
///
/// Requires `gh` CLI installed and authenticated.
/// Returns the list of successfully created issues.
pub fn create_issues(
    repo: &str,
    action_items: &[ActionItem],
    meeting_context: &str,
    meeting_date: &str,
    notes_path: &str,
) -> Vec<CreatedIssue> {
    let mut created = Vec::new();

    if action_items.is_empty() {
        log::info!("No action items to create as issues");
        return created;
    }

    log::info!(
        "Creating {} GitHub issues on {}",
        action_items.len(),
        repo
    );

    // Add a label for meeting action items
    let label = "meeting-action";
    ensure_label_exists(repo, label);

    for item in action_items {
        match create_single_issue(repo, item, meeting_context, meeting_date, notes_path, label) {
            Ok(issue) => {
                log::info!("Created issue #{}: {}", issue.number, issue.title);
                created.push(issue);
            }
            Err(e) => {
                log::error!("Failed to create issue '{}': {}", item.title, e);
            }
        }
    }

    log::info!(
        "Created {}/{} issues successfully",
        created.len(),
        action_items.len()
    );

    created
}

/// Ensure the label exists on the repo (create if missing)
fn ensure_label_exists(repo: &str, label: &str) {
    let output = std::process::Command::new("gh")
        .args([
            "label", "create", label,
            "--repo", repo,
            "--description", "Action item from a meeting",
            "--color", "0E8A16",
            "--force",
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            log::debug!("Label '{}' ensured on {}", label, repo);
        }
        _ => {
            log::debug!("Label creation skipped (may already exist)");
        }
    }
}

/// Create a single GitHub issue
fn create_single_issue(
    repo: &str,
    item: &ActionItem,
    meeting_context: &str,
    meeting_date: &str,
    notes_path: &str,
    label: &str,
) -> Result<CreatedIssue> {
    let title = if item.title.len() > 80 {
        format!("{}…", &item.title[..77])
    } else {
        item.title.clone()
    };

    let mut body = format!(
        "## Meeting Action Item\n\n\
         **Meeting:** {}\n\
         **Date:** {}\n\
         **Notes:** [{}]({})\n",
        meeting_context, meeting_date,
        std::path::Path::new(notes_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        notes_path
    );

    if let Some(ref assignee) = item.assignee {
        body.push_str(&format!("**Assigned to:** {}\n", assignee));
    }

    body.push_str(&format!("\n---\n\n{}\n", item.body));

    let args = vec![
        "issue".to_string(),
        "create".to_string(),
        "--repo".to_string(),
        repo.to_string(),
        "--title".to_string(),
        title.clone(),
        "--body".to_string(),
        body,
        "--label".to_string(),
        label.to_string(),
    ];

    let output = std::process::Command::new("gh")
        .args(&args)
        .output()
        .context("Failed to run `gh`. Is GitHub CLI installed? (brew install gh)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh issue create failed: {}", stderr);
    }

    // gh outputs the issue URL on stdout
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Extract issue number from URL (e.g. https://github.com/owner/repo/issues/42)
    let number = url
        .rsplit('/')
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    Ok(CreatedIssue {
        number,
        title,
        url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_action_items() {
        let notes = r#"
## Action Items

- [ ] **David**: Review the API design document
- [ ] **Yannick**: Set up CI pipeline for staging
- [ ] Update the documentation
- [ ] @marc Fix the login bug
- [ ] (Adrien) Prepare demo for next week
"#;
        let items = parse_action_items(notes);
        assert_eq!(items.len(), 5);
        assert_eq!(items[0].assignee.as_deref(), Some("David"));
        assert_eq!(items[0].title, "Review the API design document");
        assert_eq!(items[1].assignee.as_deref(), Some("Yannick"));
        assert_eq!(items[2].assignee, None);
        assert_eq!(items[3].assignee.as_deref(), Some("marc"));
        assert_eq!(items[4].assignee.as_deref(), Some("Adrien"));
    }

    #[test]
    fn test_parse_no_action_section() {
        let notes = "## Summary\nNothing here";
        let items = parse_action_items(notes);
        assert!(items.is_empty());
    }
}
