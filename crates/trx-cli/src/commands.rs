//! CLI command implementations

use anyhow::{Result, bail};
use colored::Colorize;
use trx_core::{
    DependencyType, Issue, IssueGraph, IssueType, Status, Store, generate_id, id::generate_child_id,
};

pub fn init(prefix: &str) -> Result<()> {
    let store = Store::init(prefix)?;
    println!(
        "{} Initialized trx in {}",
        "✓".green(),
        store.trx_dir().display()
    );
    println!("  Issue prefix: {}", prefix);
    Ok(())
}

pub fn create(
    title: &str,
    issue_type: &str,
    priority: u8,
    description: Option<String>,
    parent: Option<String>,
    json: bool,
) -> Result<()> {
    let mut store = Store::open()?;
    let prefix = store.prefix()?;

    let id = if let Some(ref parent_id) = parent {
        let child_num = store.next_child_num(parent_id);
        generate_child_id(parent_id, child_num)
    } else {
        generate_id(&prefix)
    };

    let mut issue = Issue::new(id.clone(), title.to_string());
    issue.issue_type = issue_type.parse()?;
    issue.priority = priority;
    issue.description = description;

    if let Some(ref parent_id) = parent {
        issue.add_dependency(parent_id.clone(), DependencyType::ParentChild);
    }

    store.create(issue.clone())?;

    if json {
        println!("{}", serde_json::to_string(&issue)?);
    } else {
        println!("{} Created issue: {}", "✓".green(), id);
        println!("  Title: {}", title);
        println!("  Priority: P{}", priority);
    }

    Ok(())
}

pub fn list(
    status: Option<String>,
    issue_type: Option<String>,
    all: bool,
    json: bool,
) -> Result<()> {
    let store = Store::open()?;
    let mut issues: Vec<_> = if all {
        store.list(false)
    } else {
        store.list_open()
    };

    // Filter by status
    if let Some(ref s) = status {
        let status: Status = s.parse()?;
        issues.retain(|i| i.status == status);
    }

    // Filter by type
    if let Some(ref t) = issue_type {
        let itype: IssueType = t.parse()?;
        issues.retain(|i| i.issue_type == itype);
    }

    // Sort by priority, then by creation date
    issues.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| b.created_at.cmp(&a.created_at))
    });

    if json {
        println!("{}", serde_json::to_string(&issues)?);
    } else if issues.is_empty() {
        println!("No issues found");
    } else {
        for issue in issues {
            let status_color = match issue.status {
                Status::Open => "open".white(),
                Status::InProgress => "in_progress".yellow(),
                Status::Blocked => "blocked".red(),
                Status::Closed => "closed".green(),
                Status::Tombstone => "tombstone".dimmed(),
            };
            println!(
                "{} [P{}] [{}] {} - {}",
                issue.id.cyan(),
                issue.priority,
                issue.issue_type.to_string().blue(),
                status_color,
                issue.title
            );
        }
    }

    Ok(())
}

pub fn show(id: &str, json: bool) -> Result<()> {
    let store = Store::open()?;
    let issue = store
        .get(id)
        .ok_or_else(|| anyhow::anyhow!("Issue not found: {}", id))?;

    if json {
        println!("{}", serde_json::to_string_pretty(issue)?);
    } else {
        println!("{} {}", issue.id.cyan().bold(), issue.title.bold());
        println!();
        println!("Status:   {}", issue.status);
        println!("Priority: P{}", issue.priority);
        println!("Type:     {}", issue.issue_type);
        println!("Created:  {}", issue.created_at.format("%Y-%m-%d %H:%M"));
        println!("Updated:  {}", issue.updated_at.format("%Y-%m-%d %H:%M"));

        if let Some(ref desc) = issue.description {
            println!();
            println!("{}", "Description:".bold());
            println!("{}", desc);
        }

        if !issue.dependencies.is_empty() {
            println!();
            println!("{}", "Dependencies:".bold());
            for dep in &issue.dependencies {
                println!("  {} {} {}", dep.issue_id, dep.dep_type, dep.depends_on_id);
            }
        }
    }

    Ok(())
}

pub fn update(
    id: &str,
    status: Option<String>,
    priority: Option<u8>,
    title: Option<String>,
    description: Option<String>,
    json: bool,
) -> Result<()> {
    let mut store = Store::open()?;
    let issue = store
        .get_mut(id)
        .ok_or_else(|| anyhow::anyhow!("Issue not found: {}", id))?;

    if let Some(s) = status {
        issue.status = s.parse()?;
    }
    if let Some(p) = priority {
        issue.priority = p;
    }
    if let Some(t) = title {
        issue.title = t;
    }
    if let Some(d) = description {
        issue.description = Some(d);
    }

    issue.updated_at = chrono::Utc::now();
    let issue = issue.clone();
    store.update(issue.clone())?;

    if json {
        println!("{}", serde_json::to_string(&issue)?);
    } else {
        println!("{} Updated {}", "✓".green(), id);
    }

    Ok(())
}

pub fn close(id: &str, reason: Option<String>, json: bool) -> Result<()> {
    let mut store = Store::open()?;
    let issue = store
        .get_mut(id)
        .ok_or_else(|| anyhow::anyhow!("Issue not found: {}", id))?;

    issue.close(reason);
    let issue = issue.clone();
    store.update(issue.clone())?;

    if json {
        println!("{}", serde_json::to_string(&issue)?);
    } else {
        println!("{} Closed {}", "✓".green(), id);
    }

    Ok(())
}

pub fn ready(json: bool) -> Result<()> {
    let store = Store::open()?;
    let open_issues: Vec<_> = store.list_open();
    let graph = IssueGraph::from_issues(&open_issues);
    let mut ready = graph.ready_issues(&open_issues);

    // Sort by priority
    ready.sort_by(|a, b| a.priority.cmp(&b.priority));

    if json {
        println!("{}", serde_json::to_string(&ready)?);
    } else if ready.is_empty() {
        println!("No ready issues");
    } else {
        println!("{}", "Ready issues (unblocked):".bold());
        for issue in ready {
            println!(
                "{} [P{}] [{}] - {}",
                issue.id.cyan(),
                issue.priority,
                issue.issue_type.to_string().blue(),
                issue.title
            );
        }
    }

    Ok(())
}

pub fn dep_add(id: &str, blocks: &str, json: bool) -> Result<()> {
    let mut store = Store::open()?;
    let issue = store
        .get_mut(id)
        .ok_or_else(|| anyhow::anyhow!("Issue not found: {}", id))?;

    issue.add_dependency(blocks.to_string(), DependencyType::Blocks);
    let issue = issue.clone();
    store.update(issue.clone())?;

    if json {
        println!("{}", serde_json::to_string(&issue)?);
    } else {
        println!("{} {} now blocks {}", "✓".green(), id, blocks);
    }

    Ok(())
}

pub fn dep_rm(id: &str, blocks: &str, json: bool) -> Result<()> {
    let mut store = Store::open()?;
    let issue = store
        .get_mut(id)
        .ok_or_else(|| anyhow::anyhow!("Issue not found: {}", id))?;

    issue.remove_dependency(blocks);
    let issue = issue.clone();
    store.update(issue.clone())?;

    if json {
        println!("{}", serde_json::to_string(&issue)?);
    } else {
        println!("{} {} no longer blocks {}", "✓".green(), id, blocks);
    }

    Ok(())
}

pub fn dep_tree(id: &str, _json: bool) -> Result<()> {
    let store = Store::open()?;
    let _issue = store
        .get(id)
        .ok_or_else(|| anyhow::anyhow!("Issue not found: {}", id))?;

    // TODO: Implement tree visualization
    println!("Dependency tree for {}:", id);
    println!("  (not yet implemented)");

    Ok(())
}

pub fn sync(message: Option<String>) -> Result<()> {
    let store = Store::open()?;
    let trx_dir = store.trx_dir();

    let msg = message.unwrap_or_else(|| "trx: sync issues".to_string());

    // Git add .trx/
    let output = std::process::Command::new("git")
        .args(["add", &trx_dir.to_string_lossy()])
        .output()?;

    if !output.status.success() {
        bail!(
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Git commit
    let output = std::process::Command::new("git")
        .args(["commit", "-m", &msg])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("nothing to commit") {
            println!("Nothing to sync");
            return Ok(());
        }
        bail!("git commit failed: {}", stderr);
    }

    println!("{} Synced .trx/", "✓".green());
    Ok(())
}

pub fn import(path: &str, prefix: Option<String>, json: bool) -> Result<()> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let mut store = Store::open()?;
    let new_prefix = prefix.unwrap_or_else(|| store.prefix().unwrap_or_else(|_| "trx".to_string()));

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut imported = 0;
    let mut skipped = 0;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Parse as generic JSON to handle beads fields
        let value: serde_json::Value = serde_json::from_str(&line)?;

        // Convert beads issue to trx issue
        let id = value["id"].as_str().unwrap_or("").to_string();
        if id.is_empty() {
            skipped += 1;
            continue;
        }

        // Optionally convert prefix
        let new_id = if id.starts_with("bd-") {
            id.replacen("bd-", &format!("{}-", new_prefix), 1)
        } else {
            id.clone()
        };

        let title = value["title"].as_str().unwrap_or("Untitled").to_string();
        let mut issue = Issue::new(new_id, title);

        // Map fields
        if let Some(desc) = value["description"].as_str() {
            issue.description = Some(desc.to_string());
        }
        if let Some(status) = value["status"].as_str() {
            issue.status = status.parse().unwrap_or(Status::Open);
        }
        if let Some(priority) = value["priority"].as_u64() {
            issue.priority = priority as u8;
        }
        if let Some(itype) = value["issue_type"].as_str() {
            issue.issue_type = itype.parse().unwrap_or(IssueType::Task);
        }
        if let Some(created) = value["created_at"].as_str()
            && let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created)
        {
            issue.created_at = dt.into();
        }
        if let Some(updated) = value["updated_at"].as_str()
            && let Ok(dt) = chrono::DateTime::parse_from_rfc3339(updated)
        {
            issue.updated_at = dt.into();
        }
        if let Some(closed) = value["closed_at"].as_str()
            && let Ok(dt) = chrono::DateTime::parse_from_rfc3339(closed)
        {
            issue.closed_at = Some(dt.into());
        }
        if let Some(reason) = value["close_reason"].as_str() {
            issue.close_reason = Some(reason.to_string());
        }

        // Import dependencies
        if let Some(deps) = value["dependencies"].as_array() {
            for dep in deps {
                if let (Some(depends_on), Some(dep_type)) =
                    (dep["depends_on_id"].as_str(), dep["type"].as_str())
                {
                    let dtype = match dep_type {
                        "blocks" => DependencyType::Blocks,
                        "parent-child" => DependencyType::ParentChild,
                        _ => DependencyType::Related,
                    };
                    let depends_on_id = if depends_on.starts_with("bd-") {
                        depends_on.replacen("bd-", &format!("{}-", new_prefix), 1)
                    } else {
                        depends_on.to_string()
                    };
                    issue.add_dependency(depends_on_id, dtype);
                }
            }
        }

        if store.get(&issue.id).is_some() {
            skipped += 1;
        } else {
            store.create(issue)?;
            imported += 1;
        }
    }

    if json {
        println!(r#"{{"imported": {}, "skipped": {}}}"#, imported, skipped);
    } else {
        println!(
            "{} Imported {} issues ({} skipped)",
            "✓".green(),
            imported,
            skipped
        );
    }

    Ok(())
}

pub fn purge_beads(force: bool) -> Result<()> {
    let beads_dir = std::path::Path::new(".beads");

    if !beads_dir.exists() {
        println!("No .beads directory found");
        return Ok(());
    }

    if !force {
        println!(
            "{}",
            "This will remove .beads/ directory and all beads data.".red()
        );
        println!("Make sure you have imported issues first: trx import .beads/issues.jsonl");
        println!();
        print!("Continue? [y/N] ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted");
            return Ok(());
        }
    }

    // Remove .beads directory
    std::fs::remove_dir_all(beads_dir)?;

    // Try to clean up daemon socket if exists
    let socket = std::path::Path::new(".beads/bd.sock");
    if socket.exists() {
        let _ = std::fs::remove_file(socket);
    }

    println!("{} Removed .beads/", "✓".green());
    println!("You may also want to:");
    println!("  - Remove beads from git: git rm -r .beads/");
    println!("  - Kill any running bd daemon");

    Ok(())
}

/// Output JSON schema for config file
pub fn schema() -> Result<()> {
    let schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "trx Configuration",
        "description": "Configuration file for the trx issue tracker",
        "type": "object",
        "properties": {
            "prefix": {
                "type": "string",
                "description": "Issue ID prefix (e.g., 'trx', 'myproject')",
                "default": "trx"
            },
            "default_priority": {
                "type": "integer",
                "description": "Default priority for new issues (0=critical, 1=high, 2=medium, 3=low, 4=backlog)",
                "minimum": 0,
                "maximum": 4,
                "default": 2
            },
            "default_type": {
                "type": "string",
                "enum": ["bug", "feature", "task", "epic", "chore"],
                "description": "Default issue type for new issues",
                "default": "task"
            },
            "auto_sync": {
                "type": "boolean",
                "description": "Auto-sync after mutations (git add + commit)",
                "default": false
            },
            "sync_message_template": {
                "type": "string",
                "description": "Sync commit message template. Variables: {action}, {id}, {title}",
                "default": "trx: {action} {id}"
            },
            "show_closed": {
                "type": "boolean",
                "description": "Show closed issues in list by default",
                "default": false
            },
            "editor": {
                "type": ["string", "null"],
                "description": "Editor command for editing descriptions (uses $EDITOR if not set)"
            },
            "git": {
                "type": "object",
                "properties": {
                    "auto_stage": {
                        "type": "boolean",
                        "description": "Automatically stage .trx/ after changes",
                        "default": false
                    },
                    "sync_branch": {
                        "type": ["string", "null"],
                        "description": "Branch to sync to (if different from current)"
                    }
                }
            },
            "display": {
                "type": "object",
                "properties": {
                    "colors": {
                        "type": "boolean",
                        "description": "Use colors in output",
                        "default": true
                    },
                    "date_format": {
                        "type": "string",
                        "description": "Date format for display (strftime format)",
                        "default": "%Y-%m-%d %H:%M"
                    },
                    "show_count": {
                        "type": "boolean",
                        "description": "Show issue count in list header",
                        "default": true
                    },
                    "max_title_length": {
                        "type": "integer",
                        "description": "Maximum title length before truncation",
                        "minimum": 20,
                        "default": 80
                    }
                }
            }
        }
    });
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}

/// Show current configuration
pub fn config_show(json: bool) -> Result<()> {
    let store = Store::open()?;
    let config_path = store.trx_dir().join("config.toml");
    let config = trx_core::Config::load(&config_path)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        println!("{}", "Current configuration:".bold());
        println!();
        println!("prefix = \"{}\"", config.prefix);
        println!("default_priority = {}", config.default_priority);
        println!("default_type = \"{}\"", config.default_type);
        println!("auto_sync = {}", config.auto_sync);
        println!(
            "sync_message_template = \"{}\"",
            config.sync_message_template
        );
        println!("show_closed = {}", config.show_closed);
        if let Some(ref editor) = config.editor {
            println!("editor = \"{}\"", editor);
        }
        println!();
        println!("[git]");
        println!("auto_stage = {}", config.git.auto_stage);
        if let Some(ref branch) = config.git.sync_branch {
            println!("sync_branch = \"{}\"", branch);
        }
        println!();
        println!("[display]");
        println!("colors = {}", config.display.colors);
        println!("date_format = \"{}\"", config.display.date_format);
        println!("show_count = {}", config.display.show_count);
        println!("max_title_length = {}", config.display.max_title_length);
    }

    Ok(())
}

/// Edit configuration file
pub fn config_edit() -> Result<()> {
    let store = Store::open()?;
    let config_path = store.trx_dir().join("config.toml");

    // Get editor from environment
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    let status = std::process::Command::new(&editor)
        .arg(&config_path)
        .status()?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }

    // Validate the config after editing
    match trx_core::Config::load(&config_path) {
        Ok(_) => println!("{} Configuration saved", "✓".green()),
        Err(e) => {
            println!(
                "{} Warning: Configuration may be invalid: {}",
                "!".yellow(),
                e
            );
        }
    }

    Ok(())
}

/// Reset configuration to defaults
pub fn config_reset() -> Result<()> {
    let store = Store::open()?;
    let config_path = store.trx_dir().join("config.toml");

    let default_config = trx_core::Config::default_with_comments();
    std::fs::write(&config_path, default_config)?;

    println!("{} Configuration reset to defaults", "✓".green());
    Ok(())
}

/// Get a specific config value
pub fn config_get(key: &str, json: bool) -> Result<()> {
    let store = Store::open()?;
    let config_path = store.trx_dir().join("config.toml");
    let config = trx_core::Config::load(&config_path)?;

    // Convert config to JSON for key lookup
    let config_json = serde_json::to_value(&config)?;

    // Parse key path (e.g., "display.colors" -> ["display", "colors"])
    let parts: Vec<&str> = key.split('.').collect();
    let mut value = &config_json;

    for part in &parts {
        value = value
            .get(part)
            .ok_or_else(|| anyhow::anyhow!("Config key not found: {}", key))?;
    }

    if json {
        println!("{}", serde_json::to_string(value)?);
    } else {
        match value {
            serde_json::Value::String(s) => println!("{}", s),
            serde_json::Value::Bool(b) => println!("{}", b),
            serde_json::Value::Number(n) => println!("{}", n),
            serde_json::Value::Null => println!("null"),
            _ => println!("{}", serde_json::to_string_pretty(value)?),
        }
    }

    Ok(())
}

/// Set a config value
pub fn config_set(key: &str, value: &str) -> Result<()> {
    let store = Store::open()?;
    let config_path = store.trx_dir().join("config.toml");
    let mut config = trx_core::Config::load(&config_path)?;

    // Handle top-level and nested keys
    match key {
        "prefix" => config.prefix = value.to_string(),
        "default_priority" => {
            config.default_priority = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid priority value: {}", value))?;
        }
        "default_type" => config.default_type = value.to_string(),
        "auto_sync" => {
            config.auto_sync = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean value: {}", value))?;
        }
        "sync_message_template" => config.sync_message_template = value.to_string(),
        "show_closed" => {
            config.show_closed = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean value: {}", value))?;
        }
        "editor" => config.editor = Some(value.to_string()),
        "git.auto_stage" => {
            config.git.auto_stage = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean value: {}", value))?;
        }
        "git.sync_branch" => config.git.sync_branch = Some(value.to_string()),
        "display.colors" => {
            config.display.colors = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean value: {}", value))?;
        }
        "display.date_format" => config.display.date_format = value.to_string(),
        "display.show_count" => {
            config.display.show_count = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean value: {}", value))?;
        }
        "display.max_title_length" => {
            config.display.max_title_length = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid integer value: {}", value))?;
        }
        _ => bail!("Unknown config key: {}", key),
    }

    config.save(&config_path)?;
    println!("{} Set {} = {}", "✓".green(), key, value);

    Ok(())
}
