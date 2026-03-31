use anyhow::Result;
use serde::Serialize;

use crate::axcli;
use crate::output::OutputFormat;

#[derive(Debug, Serialize)]
struct AXNodeInfo {
    role: String,
    classes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<(f64, f64)>,
    locator_hint: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<AXNodeInfo>,
}

pub fn inspect(locator: Option<&str>, depth: usize, format: OutputFormat) -> Result<()> {
    let node = if let Some(loc) = locator {
        axcli::locate(loc)?
    } else {
        axcli::chrome_root()?
    };

    let info = build_node_info(&node, depth);

    match format {
        OutputFormat::Text => {
            print_tree(&info, "", true);
        }
        _ => crate::output::print_value(&info, format),
    }

    Ok(())
}

fn build_node_info(node: &axcli::AXNode, remaining_depth: usize) -> AXNodeInfo {
    let role = node
        .role()
        .unwrap_or_default()
        .strip_prefix("AX")
        .unwrap_or(&node.role().unwrap_or_default())
        .to_lowercase();
    let classes = node.dom_classes();
    let title = node.title().filter(|s| !s.is_empty());
    let value_raw = node.value().filter(|s| !s.is_empty());
    let description = node.description().filter(|s| !s.is_empty());
    let text = node.text(1).trim().to_string();

    // Build a locator hint that can be used directly in this project
    let locator_hint = build_locator_hint(&role, &classes, &title);

    let children = if remaining_depth > 0 {
        node.children()
            .iter()
            .map(|child| build_node_info(child, remaining_depth - 1))
            .collect()
    } else {
        vec![]
    };

    let position = node.position();
    let size = node.size();

    AXNodeInfo {
        role,
        classes,
        title,
        value: value_raw,
        description,
        text,
        position,
        size,
        locator_hint,
        children,
    }
}

/// Build a locator hint string like "group.note-item" or "textfield#search-input"
fn build_locator_hint(role: &str, classes: &[String], title: &Option<String>) -> String {
    let role_part = match role {
        "group" | "statictext" | "image" | "list" | "cell" | "row" | "column"
        | "scrollarea" | "unknown" | "layoutarea" | "layoutitem" | "" => None,
        _ => Some(role.to_string()),
    };

    let class_part = classes.first().map(|c| format!(".{}", c));
    let title_part = title
        .as_ref()
        .filter(|t| t.len() < 30)
        .map(|t| format!("[title=\"{}\"]", t));

    match (role_part, class_part, title_part) {
        (Some(r), Some(c), _) => format!("{}{}", r, c),
        (None, Some(c), _) => c,
        (Some(r), None, Some(t)) => format!("{}{}", r, t),
        (Some(r), None, None) => r,
        (None, None, Some(t)) => t,
        (None, None, None) => role.to_string(),
    }
}

fn print_tree(info: &AXNodeInfo, prefix: &str, is_last: bool) {
    let connector = if prefix.is_empty() {
        ""
    } else if is_last {
        "└─ "
    } else {
        "├─ "
    };

    // Build display string
    let mut display = info.locator_hint.clone();

    // Add text content if present (truncated)
    if !info.text.is_empty() {
        let truncated = if info.text.chars().count() > 60 {
            let s: String = info.text.chars().take(57).collect();
            format!("{}...", s)
        } else {
            info.text.clone()
        };
        display = format!("{}  \"{}\"", display, truncated);
    }

    println!("{}{}{}", prefix, connector, display);

    // Print children
    let child_prefix = if prefix.is_empty() {
        "  ".to_string()
    } else if is_last {
        format!("{}   ", prefix)
    } else {
        format!("{}│  ", prefix)
    };

    for (i, child) in info.children.iter().enumerate() {
        let is_last_child = i == info.children.len() - 1;
        print_tree(child, &child_prefix, is_last_child);
    }
}
