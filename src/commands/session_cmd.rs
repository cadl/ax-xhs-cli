use anyhow::Result;
use crate::output::OutputFormat;
use crate::session;
use crate::axcli;

pub fn start(name: &str, format: OutputFormat) -> Result<()> {
    let session = session::start_session(name)?;
    match format {
        OutputFormat::Text => {
            println!("Session '{}' started", session.id);
            println!("  Page:   {}", session.page_type);
            if let Some(tab_id) = session.tab_id {
                println!("  Tab ID: {}", tab_id);
            }
        }
        _ => crate::output::print_value(&serde_json::json!({
            "id": session.id,
            "page_type": session.page_type,
            "tab_id": session.tab_id,
        }), format),
    }
    Ok(())
}

pub fn list(format: OutputFormat) -> Result<()> {
    let sessions = session::list_sessions()?;
    match format {
        OutputFormat::Text => {
            if sessions.is_empty() {
                println!("没有活跃的 session");
                return Ok(());
            }
            for s in &sessions {
                let child = if !s.child_tabs.is_empty() {
                    format!(" [{}个子tab]", s.child_tabs.len())
                } else {
                    String::new()
                };
                println!(
                    "  {} | {} | {}{} | results: {}",
                    s.id,
                    s.page_type,
                    s.scene_param("keyword").unwrap_or("-"),
                    child,
                    s.results.len()
                );
            }
        }
        _ => crate::output::print_value(&sessions, format),
    }
    Ok(())
}

pub fn end(session_id: Option<&str>, format: OutputFormat) -> Result<()> {
    let id = session_id.ok_or_else(|| anyhow::anyhow!(
        "请通过 session end <NAME> 指定要结束的 session"
    ))?;
    let mut session = session::Session::load(id)?;

    // Close child tabs first (ignore errors — tabs may already be closed)
    let _ = session.close_all_child_tabs();

    // Close the main tab by ID (ignore errors — tab may already be closed)
    if let Some(ref tab_id) = session.tab_id {
        let _ = axcli::close_tab_by_id(tab_id);
    }

    let id = session.id.clone();
    session.delete()?;
    crate::output::print_action_result("session_end", true, &format!("Session '{}' ended", id), format);
    Ok(())
}

pub fn status(session_id: Option<&str>, format: OutputFormat) -> Result<()> {
    let mut session = session::get_active_session(session_id)?;
    let page_type = session.detect_page_type()?;
    let has_results = !session.results.is_empty();

    match format {
        OutputFormat::Text => {
            println!("Session:  {}", session.id);
            println!("Page:     {}", page_type);
            if let Some(ref tab_id) = session.tab_id {
                println!("Tab ID:   {}", tab_id);
            }
            if !session.child_tabs.is_empty() {
                println!("子 tab:");
                for (i, child) in session.child_tabs.iter().enumerate() {
                    println!(
                        "  [{}] {} (xhs_id: {}) - {} 条笔记",
                        i,
                        child.nickname,
                        child.xhs_id,
                        child.results.len()
                    );
                }
            }
            if let Some(kw) = &session.scene_param("keyword").map(|s| s.to_string()) {
                println!("Keyword:  {}", kw);
            }
            if has_results {
                println!("Results:  {} items", session.results.len());
                for r in &session.results {
                    println!("  [{}] {} - {} (❤{})", r.index, r.title, r.author, r.likes);
                }
            }

            // Show available actions
            println!();
            println!("{}", page_type.next_step_hint(has_results));
            println!();
            println!("可用命令:");
            for action in page_type.available_actions(has_results) {
                println!(
                    "  {:<18} {}  (例: {})",
                    action.command, action.description, action.example
                );
            }
        }
        _ => crate::output::print_value(&session, format),
    }
    Ok(())
}
