use anyhow::Result;

use crate::output::OutputFormat;
use crate::{axcli, output, session};
use super::actions::{NoteAction, dispatch_note_action};

/// user-profile command dispatcher
pub fn user_profile(
    session_id: Option<&str>,
    name: Option<&str>,
    size: usize,
    action: Option<UserProfileAction>,
    format: OutputFormat,
) -> Result<()> {
    let mut session = session::get_active_session(session_id)?;

    match action {
        Some(UserProfileAction::List) => {
            if session.child_tabs.is_empty() {
                output::print_action_result("user-profile list", true, "没有已打开的用户 tab", format);
                return Ok(());
            }
            match format {
                OutputFormat::Text => {
                    for (i, child) in session.child_tabs.iter().enumerate() {
                        println!(
                            "[{}] {} (xhs_id: {}) - {} 条笔记",
                            i, child.nickname, child.xhs_id, child.results.len()
                        );
                    }
                }
                _ => output::print_value(&session.child_tabs, format),
            }
            Ok(())
        }
        Some(UserProfileAction::Close { name: close_name }) => {
            let nickname = session
                .find_child_tab(&close_name)
                .map(|c| c.nickname.clone())
                .ok_or_else(|| anyhow::anyhow!("未找到用户: {}", close_name))?;
            session.close_child_tab(&close_name)?;
            output::print_action_result(
                "user-profile close",
                true,
                &format!("已关闭用户 tab: {}", nickname),
                format,
            );
            Ok(())
        }
        Some(UserProfileAction::NoteAction(note_action)) => {
            let name = name.ok_or_else(|| anyhow::anyhow!("请通过 --name 指定用户"))?;
            let child_idx = session
                .child_tabs
                .iter()
                .position(|t| t.nickname == name)
                .or_else(|| name.parse::<usize>().ok().filter(|&i| i < session.child_tabs.len()))
                .ok_or_else(|| {
                    anyhow::anyhow!("未找到用户: {}。使用 search/feeds show-user 先打开用户页", name)
                })?;
            let tab_id = session.child_tabs[child_idx].tab_id.clone();
            let mut results = std::mem::take(&mut session.child_tabs[child_idx].results);

            // Switch to child tab for the action
            axcli::switch_to_tab(&tab_id)?;
            std::thread::sleep(std::time::Duration::from_millis(500));

            let result = dispatch_note_action(&note_action, &mut session, &mut results, format);

            session.child_tabs[child_idx].results = results;
            // Switch back to main tab
            session.activate_tab()?;
            session.save()?;
            result
        }
        None => {
            // No subcommand: show user profile + notes
            let name = name.ok_or_else(|| anyhow::anyhow!("请通过 --name 指定用户"))?;
            let child_idx = session
                .child_tabs
                .iter()
                .position(|t| t.nickname == name)
                .or_else(|| name.parse::<usize>().ok().filter(|&i| i < session.child_tabs.len()))
                .ok_or_else(|| {
                    anyhow::anyhow!("未找到用户: {}。使用 search/feeds show-user 先打开用户页", name)
                })?;

            let child = &session.child_tabs[child_idx];
            if child.results.len() >= size {
                let end = std::cmp::min(size, child.results.len());
                let slice = &child.results[..end];
                let profile = crate::parser::UserProfile {
                    nickname: child.nickname.clone(),
                    xhs_id: child.xhs_id.clone(),
                    ..Default::default()
                };
                output::print_user(&profile, slice, format);
                return Ok(());
            }

            let tab_id = child.tab_id.clone();
            axcli::switch_to_tab(&tab_id)?;
            std::thread::sleep(std::time::Duration::from_millis(500));

            let profile = axcli::locate_opt("#userPageContainer")
                .map(|c| crate::parser::extract_user_profile(&c))
                .unwrap_or_else(|| crate::parser::UserProfile {
                    nickname: session.child_tabs[child_idx].nickname.clone(),
                    xhs_id: session.child_tabs[child_idx].xhs_id.clone(),
                    ..Default::default()
                });

            let existing = &session.child_tabs[child_idx].results;
            let notes = super::actions::scroll_and_collect_user_notes_pub(existing, size)?;

            let end = std::cmp::min(size, notes.len());
            let slice = &notes[..end];
            output::print_user(&profile, slice, format);

            let child = &mut session.child_tabs[child_idx];
            child.results = notes;
            child.nickname = profile.nickname;
            child.xhs_id = profile.xhs_id;

            session.activate_tab()?;
            session.save()?;
            Ok(())
        }
    }
}

/// Subcommands for user-profile
#[derive(clap::Subcommand, Debug)]
pub enum UserProfileAction {
    /// 列出所有已打开的用户 tab
    List,
    /// 关闭用户 tab
    Close {
        /// 用户标识（昵称或索引）
        name: String,
    },
    /// Note actions (flattened)
    #[command(flatten)]
    NoteAction(NoteAction),
}
