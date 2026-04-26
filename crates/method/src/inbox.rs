// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! INBOX command: capture raw ideas as backlog files.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::workspace::MethodWorkspace;

/// Convert a title into a safe inbox filename.
pub fn filename_from_title(title: &str) -> Result<String, String> {
    let normalized = normalize_title(title)?;
    let mut slug = String::new();
    let mut needs_dash = false;

    for ch in normalized.chars() {
        if ch.is_ascii_alphanumeric() {
            if needs_dash && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch.to_ascii_lowercase());
            needs_dash = false;
        } else if !slug.is_empty() {
            needs_dash = true;
        }
    }

    if slug.is_empty() {
        return Err("title must contain at least one ASCII letter or digit".to_string());
    }

    Ok(format!("{slug}.md"))
}

/// Create a METHOD inbox item and return the created file path.
pub fn create_inbox_item(workspace: &MethodWorkspace, title: &str) -> Result<PathBuf, String> {
    let normalized = normalize_title(title)?;
    let filename = filename_from_title(&normalized)?;
    let inbox_dir = workspace.backlog_root().join("inbox");
    fs::create_dir_all(&inbox_dir).map_err(|e| {
        format!(
            "failed to create inbox directory {}: {e}",
            inbox_dir.display()
        )
    })?;

    let path = inbox_dir.join(filename);
    let content = render_inbox_item(&normalized);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "refusing to overwrite existing inbox item: {}",
                    path.display()
                )
            } else {
                format!("failed to create inbox item {}: {e}", path.display())
            }
        })?;

    if let Err(e) = file.write_all(content.as_bytes()) {
        let _ = fs::remove_file(&path);
        return Err(format!(
            "failed to write inbox item {}: {e}",
            path.display()
        ));
    }

    Ok(path)
}

fn normalize_title(title: &str) -> Result<String, String> {
    let normalized = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Err("title must not be empty".to_string());
    }
    Ok(normalized)
}

fn render_inbox_item(title: &str) -> String {
    format!(
        "\
<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# {title}

> **Milestone:** Inbox | **Priority:** Unscheduled

Captured with `cargo xtask method inbox`.

## Note

{title}
"
    )
}
