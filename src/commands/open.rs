use anyhow::{bail, Result};

use crate::axcli::DominantColor;
use crate::output::OutputFormat;
use crate::session::{ChildTab, SearchResult};
use crate::{axcli, output, parser, session};
use super::actions::extract_and_print_detail;
use super::search::scroll_and_collect;

/// Subcommands for open-note scene
#[derive(clap::Subcommand, Debug)]
pub enum OpenNoteAction {
    /// 点赞笔记
    #[command(name = "like-note")]
    LikeNote,
    /// 取消点赞
    #[command(name = "unlike-note")]
    UnlikeNote,
    /// 收藏笔记
    #[command(name = "favorite-note")]
    FavoriteNote,
    /// 取消收藏
    #[command(name = "unfavorite-note")]
    UnfavoriteNote,
    /// 查看笔记评论
    #[command(name = "show-comments")]
    ShowComments {
        #[arg(long, default_value = "20")]
        size: usize,
    },
}

/// Open a note by URL, optionally execute an action
pub fn open_note(
    session_id: Option<&str>,
    url: &str,
    action: Option<OpenNoteAction>,
    format: OutputFormat,
) -> Result<()> {
    if !url.contains("xiaohongshu.com") {
        bail!("无效的笔记 URL: {}", url);
    }

    let mut session = session::get_active_session(session_id)?;

    // Check if we're already on this note page (same URL scene param)
    let same_note = session.page_type == session::PageType::NoteDetail
        && session.scene_param("url").is_some_and(|saved| saved == url);

    if !same_note {
        axcli::navigate_open(url)?;

        // Wait for note page to load
        let mut loaded = false;
        for _ in 0..5 {
            if axcli::exists("#noteContainer") || axcli::exists("#detail-title") {
                loaded = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        if !loaded {
            bail!("无法加载笔记页面，请检查 URL 是否正确（需要包含 xsec_token 参数）");
        }

        session.page_type = session::PageType::NoteDetail;
        session.clear_scene_params();
        session.set_scene_param("url", url);
        session.save()?;
    }

    match action {
        None => {
            // Just show note detail
            let _ = extract_and_print_detail(Some(url), format)?;
        }
        Some(OpenNoteAction::LikeNote) => {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let already_liked = axcli::detect_element_color(".engage-bar >> .like-wrapper >> .like-icon")
                .map(|c| c == DominantColor::Red)
                .unwrap_or(false);
            if already_liked {
                output::print_action_result("like", false, "已经点赞过了", format);
            } else {
                axcli::human_click(".engage-bar >> .like-wrapper")?;
                std::thread::sleep(std::time::Duration::from_millis(500));
                output::print_action_result("like", true, "点赞成功", format);
            }
        }
        Some(OpenNoteAction::UnlikeNote) => {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let already_liked = axcli::detect_element_color(".engage-bar >> .like-wrapper >> .like-icon")
                .map(|c| c == DominantColor::Red)
                .unwrap_or(false);
            if !already_liked {
                output::print_action_result("unlike", false, "当前未点赞，无需取消", format);
            } else {
                axcli::human_click(".engage-bar >> .like-wrapper")?;
                std::thread::sleep(std::time::Duration::from_millis(500));
                output::print_action_result("unlike", true, "已取消点赞", format);
            }
        }
        Some(OpenNoteAction::FavoriteNote) => {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let collect_icon = if axcli::exists("#note-page-collect-board-guide") {
                "#note-page-collect-board-guide >> .collect-icon"
            } else {
                ".engage-bar >> .collect-wrapper >> .collect-icon"
            };
            let already = axcli::detect_element_color(collect_icon)
                .map(|c| c == DominantColor::Yellow)
                .unwrap_or(false);
            if already {
                output::print_action_result("favorite", false, "已经收藏过了", format);
            } else {
                let selector = if axcli::exists("#note-page-collect-board-guide") {
                    "#note-page-collect-board-guide"
                } else {
                    ".engage-bar >> .collect-wrapper"
                };
                axcli::human_click(selector)?;
                std::thread::sleep(std::time::Duration::from_millis(500));
                output::print_action_result("favorite", true, "收藏成功", format);
            }
        }
        Some(OpenNoteAction::UnfavoriteNote) => {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let collect_icon = if axcli::exists("#note-page-collect-board-guide") {
                "#note-page-collect-board-guide >> .collect-icon"
            } else {
                ".engage-bar >> .collect-wrapper >> .collect-icon"
            };
            let already = axcli::detect_element_color(collect_icon)
                .map(|c| c == DominantColor::Yellow)
                .unwrap_or(false);
            if !already {
                output::print_action_result("unfavorite", false, "当前未收藏，无需取消", format);
            } else {
                let selector = if axcli::exists("#note-page-collect-board-guide") {
                    "#note-page-collect-board-guide"
                } else {
                    ".engage-bar >> .collect-wrapper"
                };
                axcli::human_click(selector)?;
                std::thread::sleep(std::time::Duration::from_millis(500));
                output::print_action_result("unfavorite", true, "已取消收藏", format);
            }
        }
        Some(OpenNoteAction::ShowComments { size }) => {
            let comments = scroll_and_collect_comments(size)?;
            let total = axcli::locate_opt(".comments-el >> .total")
                .map(|n| n.text(2).trim().to_string())
                .unwrap_or_default();
            let end = std::cmp::min(size, comments.len());
            let display = &comments[..end];
            output::print_comments(display, format, &total);
        }
    }

    Ok(())
}

fn scroll_and_collect_comments(target_count: usize) -> Result<Vec<parser::Comment>> {
    let mut all_comments = Vec::new();

    for _ in 0..50 {
        if let Some(container) = axcli::locate_opt(".comments-el") {
            let visible = parser::extract_comments(&container);
            if visible.len() > all_comments.len() {
                all_comments = visible;
            }
        }
        if all_comments.len() >= target_count {
            break;
        }
        if axcli::exists(".note-scroller") {
            axcli::scroll_element_down(".note-scroller", 400)?;
            std::thread::sleep(std::time::Duration::from_secs(1));
        } else {
            break;
        }
        let new_count = axcli::locate_opt(".comments-el")
            .map(|c| parser::extract_comments(&c).len())
            .unwrap_or(0);
        if new_count <= all_comments.len() {
            break;
        }
    }
    Ok(all_comments)
}

/// Open a user profile by URL (as a child tab, like normal user flow)
pub fn open_user(
    session_id: Option<&str>,
    url: &str,
    size: usize,
    format: OutputFormat,
) -> Result<()> {
    if !url.contains("xiaohongshu.com") {
        bail!("无效的用户 URL: {}", url);
    }

    let mut session = session::get_active_session(session_id)?;

    // Ensure we're on the homepage first (so user page opens as a child tab)
    if session.page_type != session::PageType::Home {
        axcli::human_click("link#link-guide")?;
        std::thread::sleep(std::time::Duration::from_secs(2));
        if !axcli::exists("#exploreFeeds") {
            axcli::navigate_open("https://www.xiaohongshu.com/explore")?;
        }
        session.page_type = session::PageType::Home;
        session.save()?;
    }

    // Open URL in a new tab
    axcli::open_url(url)?;

    let child_tab_id = axcli::get_active_tab_id()?;
    let is_new_tab = Some(&child_tab_id) != session.tab_id.as_ref();

    // Wait for user page to load
    let mut loaded = false;
    for _ in 0..5 {
        if axcli::exists("#userPageContainer") {
            loaded = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    if !loaded {
        // Page didn't load — close the tab and report error
        if is_new_tab {
            let _ = axcli::close_tab_by_id(&child_tab_id);
        }
        session.activate_tab()?;
        bail!("无法加载用户主页，请检查 URL 是否正确");
    }

    if let Some(container) = axcli::locate_opt("#userPageContainer") {
        let profile = parser::extract_user_profile(&container);

        // Check if profile is valid (nickname empty = user not found)
        if profile.nickname.is_empty() {
            if is_new_tab {
                let _ = axcli::close_tab_by_id(&child_tab_id);
            }
            session.activate_tab()?;
            bail!("用户不存在或页面加载异常，请检查 URL");
        }

        // Check if we already have a child tab for this user
        if let Some(pos) = session.child_tabs.iter().position(|t| t.nickname == profile.nickname) {
            let end = std::cmp::min(size, session.child_tabs[pos].results.len());
            let slice = &session.child_tabs[pos].results[..end];
            output::print_user(&profile, slice, format);
            // Close duplicate tab
            if is_new_tab && child_tab_id != session.child_tabs[pos].tab_id {
                let _ = axcli::close_tab_by_id(&child_tab_id);
            }
        } else {
            // New user
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
                    url: String::new(),
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
        bail!("无法加载用户主页");
    }

    // Switch back to main tab
    session.activate_tab()?;
    session.save()?;
    Ok(())
}
