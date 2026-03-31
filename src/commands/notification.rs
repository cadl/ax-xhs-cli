use anyhow::{bail, Result};

use crate::output::OutputFormat;
use crate::session::{ChildTab, SearchResult};
use crate::{axcli, output, parser, session};
use super::search::scroll_and_collect;

/// Ensure we're on the notification page
fn ensure_notification_page(session: &mut session::Session) -> Result<()> {
    if session.page_type != session::PageType::Notification {
        axcli::human_click(".link-wrapper:has-text(\"通知\")")?;
        std::thread::sleep(std::time::Duration::from_secs(3));
        session.page_type = session::PageType::Notification;
        session.save()?;
    }
    Ok(())
}

/// notification command dispatcher
pub fn notification(
    session_id: Option<&str>,
    tab: Option<&str>,
    action: Option<NotificationAction>,
    format: OutputFormat,
) -> Result<()> {
    let mut session = session::get_active_session(session_id)?;

    ensure_notification_page(&mut session)?;

    // Resolve tab: use passed value, or fall back to saved scene param
    let tab = tab
        .map(|s| s.to_string())
        .or_else(|| session.scene_param("tab").map(|s| s.to_string()));

    if let Some(ref tab_name) = tab {
        // Check for scene param mismatches when doing a subcommand
        if action.is_some() && !session.scene_params.is_empty() {
            let mismatches = session.check_scene_params(&[("tab", Some(tab_name))]);
            if !mismatches.is_empty() {
                for (key, saved, new) in &mismatches {
                    eprintln!("错误: 场景参数不一致: {} = \"{}\"（命令指定: \"{}\"）", key, saved, new);
                }
                eprintln!("\n如需切换 tab，请先不带子命令执行 notification 进入新场景");
                bail!("场景参数不一致");
            }
        }
        // Switch to the tab
        let selector = format!(".reds-tab-item:has-text(\"{}\")", tab_name);
        axcli::human_click(&selector)?;
        std::thread::sleep(std::time::Duration::from_secs(2));
        // Save scene param
        session.set_scene_param("tab", tab_name);
        session.save()?;
    }

    match action {
        Some(NotificationAction::ShowUser { index, size }) => {
            if tab.is_none() {
                bail!("show-user 需要指定 --scene-tab 参数（评论和@、赞和收藏、新增关注），以确定通知列表上下文");
            }
            show_user_from_notification(&mut session, index, size, format)
        }
        None => {
            let container = axcli::locate(".layout")?;
            let items = parser::extract_notifications(&container);
            output::print_notifications(&items, format);
            Ok(())
        }
    }
}

/// Subcommands for notification
#[derive(clap::Subcommand, Debug)]
pub enum NotificationAction {
    /// 从通知打开用户主页
    #[command(name = "show-user")]
    ShowUser {
        /// 通知索引（0-based）
        index: usize,
        #[arg(long, default_value = "20")]
        size: usize,
    },
}

fn show_user_from_notification(
    session: &mut session::Session,
    index: usize,
    size: usize,
    format: OutputFormat,
) -> Result<()> {
    let layout = axcli::locate(".layout")?;
    let hint_count = layout.locate_all(".interaction-hint").len();
    if index >= hint_count {
        bail!(
            "索引超出范围: {}（共 {} 条通知，索引 0-{}）",
            index,
            hint_count,
            hint_count.saturating_sub(1)
        );
    }

    // Find avatar links by iterating children
    let avatar_positions: Vec<usize> = layout
        .children()
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.role().as_deref() == Some("AXLink") && c.has_dom_class("user-avatar")
        })
        .map(|(i, _)| i)
        .collect();

    if index >= avatar_positions.len() {
        bail!("无法定位第 {} 条通知的用户头像", index);
    }

    let avatar = layout
        .child(avatar_positions[index])
        .ok_or_else(|| anyhow::anyhow!("无法获取用户头像元素"))?;
    let (px, py) = avatar
        .position()
        .ok_or_else(|| anyhow::anyhow!("无法获取头像位置"))?;
    let (sx, sy) = avatar
        .size()
        .ok_or_else(|| anyhow::anyhow!("无法获取头像大小"))?;
    let center = core_graphics::geometry::CGPoint::new(px + sx / 2.0, py + sy / 2.0);
    crate::mouse::move_to(center, None)?;
    std::thread::sleep(std::time::Duration::from_millis(rand::random_range(500..1500)));
    crate::mouse::click_at_current()?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let child_tab_id = axcli::get_active_tab_id()?;
    let is_new_tab = Some(&child_tab_id) != session.tab_id.as_ref();

    for _ in 0..3 {
        if axcli::exists("#userPageContainer") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    if let Some(container) = axcli::locate_opt("#userPageContainer") {
        let profile = parser::extract_user_profile(&container);

        if let Some(pos) = session.child_tabs.iter().position(|t| t.nickname == profile.nickname) {
            let end = std::cmp::min(size, session.child_tabs[pos].results.len());
            let slice = &session.child_tabs[pos].results[..end];
            output::print_user(&profile, slice, format);

            if is_new_tab && child_tab_id != session.child_tabs[pos].tab_id {
                let _ = axcli::close_tab_by_id(&child_tab_id);
            }
        } else {
            let cards = scroll_and_collect("#userPageContainer", size, 50)
                .unwrap_or_default();
            let notes: Vec<SearchResult> = cards
                .iter()
                .enumerate()
                .map(|(i, card)| SearchResult {
                    index: i,
                    title: card.title.clone(),
                    author: card.author.clone(),
                    likes: card.likes.clone(),
                })
                .collect();

            let end = std::cmp::min(size, notes.len());
            let slice = &notes[..end];
            output::print_user(&profile, slice, format);

            if is_new_tab {
                session.child_tabs.push(ChildTab {
                    tab_id: child_tab_id,
                    nickname: profile.nickname,
                    xhs_id: profile.xhs_id,
                    results: notes,
                });
            }
        }
    } else {
        eprintln!("警告: 无法加载用户主页");
    }

    session.activate_tab()?;
    session.save()?;
    Ok(())
}
