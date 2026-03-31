//! Shared note action handlers used across search/feeds/user-profile scenes.

use anyhow::{bail, Result};
use clap::Subcommand;

use crate::axcli::{self, DominantColor};
use crate::output::OutputFormat;
use crate::session::{ChildTab, SearchResult, Session};
use crate::{output, parser};
use super::search::scroll_and_collect;

/// Common note actions shared across search, feeds, user-profile scenes.
#[derive(Subcommand, Debug)]
pub enum NoteAction {
    /// 查看笔记详情
    #[command(name = "show-note")]
    ShowNote { index: usize },
    /// 查看笔记作者主页
    #[command(name = "show-user")]
    ShowUser {
        index: usize,
        #[arg(long, default_value = "20")]
        size: usize,
    },
    /// 点赞笔记
    #[command(name = "like-note")]
    LikeNote { index: usize },
    /// 取消点赞
    #[command(name = "unlike-note")]
    UnlikeNote { index: usize },
    /// 收藏笔记
    #[command(name = "favorite-note")]
    FavoriteNote { index: usize },
    /// 取消收藏
    #[command(name = "unfavorite-note")]
    UnfavoriteNote { index: usize },
    /// 评论笔记
    #[command(name = "comment-note")]
    CommentNote {
        index: usize,
        #[arg(long, short)]
        content: String,
    },
    /// 查看笔记评论
    #[command(name = "show-comments")]
    ShowComments {
        index: usize,
        #[arg(long, default_value = "20")]
        size: usize,
    },
}

/// Dispatch a NoteAction on the current page's results.
/// `results` is the active result list (from session.results or child_tab.results).
/// New cards discovered during scrolling are appended to `results`.
pub fn dispatch_note_action(
    action: &NoteAction,
    session: &mut Session,
    results: &mut Vec<SearchResult>,
    format: OutputFormat,
) -> Result<()> {
    match action {
        NoteAction::ShowNote { index } => show_note(results, *index, format),
        NoteAction::ShowUser { index, size } => {
            show_user(session, results, *index, *size, format)
        }
        NoteAction::LikeNote { index } => like_note(results, *index, false, format),
        NoteAction::UnlikeNote { index } => like_note(results, *index, true, format),
        NoteAction::FavoriteNote { index } => favorite_note(results, *index, false, format),
        NoteAction::UnfavoriteNote { index } => favorite_note(results, *index, true, format),
        NoteAction::CommentNote { index, content } => {
            comment_note(results, *index, content, format)
        }
        NoteAction::ShowComments { index, size } => {
            show_comments(results, *index, *size, format)
        }
    }
}

fn validate_index(results: &[SearchResult], index: usize) -> Result<()> {
    if results.is_empty() {
        bail!("当前没有结果");
    }
    if index >= results.len() {
        bail!(
            "索引超出范围: {}（共 {} 条结果，索引 0-{}）",
            index,
            results.len(),
            results.len() - 1
        );
    }
    Ok(())
}

/// Detect the current scrollable container for note items.
fn note_container() -> &'static str {
    if axcli::exists("#userPageContainer") {
        "#userPageContainer"
    } else {
        ".feeds-container"
    }
}

/// Scroll to bring the target note-item into the viewport by matching against
/// session results.  Compares visible cards with the results list to determine
/// the current scroll position, then scrolls up/down as needed.
/// New cards discovered during scrolling are appended to `results`.
///
/// Returns the center point of the target card's AXNode for direct clicking,
/// avoiding nth= locator mismatch with phantom DOM elements.
fn scroll_to_note(
    results: &mut Vec<SearchResult>,
    index: usize,
) -> Result<core_graphics::geometry::CGPoint> {
    use std::collections::HashSet;

    let target_title = results[index].title.clone();
    let target_author = results[index].author.clone();
    let container = note_container();
    let mut last_range: Option<(usize, usize)> = None;
    let mut stall_count = 0;
    let mut seen: HashSet<(String, String)> = results
        .iter()
        .map(|r| (r.title.clone(), r.author.clone()))
        .collect();

    for _ in 0..80 {
        let container_node = axcli::locate(container)?;
        let card_nodes = container_node.locate_all(".note-item");
        let visible = parser::extract_note_cards(&container_node);

        // Accumulate new cards into results (HashSet dedup)
        for card in &visible {
            let key = (card.title.clone(), card.author.clone());
            if seen.insert(key) {
                results.push(SearchResult {
                    index: results.len(),
                    title: card.title.clone(),
                    author: card.author.clone(),
                    likes: card.likes.clone(),
                });
            }
        }

        // Find target among visible cards and return its AXNode center point.
        // We walk card_nodes and visible in tandem (extract_note_cards may
        // skip nodes that have neither title nor author).
        for node in &card_nodes {
            let title = node
                .locate(".title")
                .map(|n| n.text(1).trim().to_string())
                .unwrap_or_default();
            let author = node
                .locate(".name")
                .map(|n| n.text(1).trim().to_string())
                .or_else(|| node.locate(".author").map(|n| n.text(1).trim().to_string()))
                .unwrap_or_default();
            if title.is_empty() && author.is_empty() {
                continue; // skip phantom nodes (same filter as extract_note_cards)
            }
            if title == target_title && author == target_author {
                // Ensure the card is near the viewport center
                if let (Some((px, py)), Some((sx, sy))) = (node.position(), node.size()) {
                    let center =
                        core_graphics::geometry::CGPoint::new(px + sx / 2.0, py + sy / 2.0);
                    // Scroll to bring into viewport if needed
                    let (_, vp_top, vp_bottom) = axcli::chrome_viewport()?;
                    let vp_center = (vp_top + vp_bottom) / 2.0;
                    let tolerance = (vp_bottom - vp_top) / 4.0;
                    if (center.y - vp_center).abs() > tolerance {
                        let nudge = (center.y - vp_center) as i32;
                        axcli::scroll_on_container(container, nudge)?;
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        // Re-locate after scroll
                        let container2 = axcli::locate(container)?;
                        let nodes2 = container2.locate_all(".note-item");
                        for n2 in &nodes2 {
                            let t2 = n2.locate(".title").map(|n| n.text(1).trim().to_string()).unwrap_or_default();
                            let a2 = n2.locate(".name").map(|n| n.text(1).trim().to_string())
                                .or_else(|| n2.locate(".author").map(|n| n.text(1).trim().to_string()))
                                .unwrap_or_default();
                            if t2 == target_title && a2 == target_author {
                                if let (Some((px2, py2)), Some((sx2, sy2))) = (n2.position(), n2.size()) {
                                    return Ok(core_graphics::geometry::CGPoint::new(
                                        px2 + sx2 / 2.0,
                                        py2 + sy2 / 2.0,
                                    ));
                                }
                            }
                        }
                    }
                    return Ok(center);
                }
            }
        }

        // Match visible cards against results to determine current scroll position
        let visible_indices: Vec<usize> = visible
            .iter()
            .filter_map(|card| {
                results
                    .iter()
                    .position(|r| r.title == card.title && r.author == card.author)
            })
            .collect();

        if visible_indices.is_empty() {
            axcli::scroll_to_top(container)?;
            last_range = None;
            stall_count = 0;
            continue;
        }

        let min_visible = *visible_indices.iter().min().unwrap();
        let max_visible = *visible_indices.iter().max().unwrap();

        let current_range = (min_visible, max_visible);
        if last_range == Some(current_range) {
            stall_count += 1;
        } else {
            stall_count = 0;
        }
        last_range = Some(current_range);

        if index < min_visible {
            axcli::scroll_on_container(container, -500)?;
        } else if index > max_visible {
            axcli::scroll_on_container(container, 500)?;
        } else {
            axcli::scroll_on_container(container, 200)?;
        }

        if stall_count >= 5 {
            bail!(
                "滚动无进展，无法定位笔记 (index {})，可能该笔记已不在页面中",
                index
            );
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    bail!("无法定位笔记 (index {})", index);
}

fn open_note_modal(results: &mut Vec<SearchResult>, index: usize) -> Result<()> {
    validate_index(results, index)?;
    // Close any existing note modal first to ensure we open the correct note
    if axcli::exists("#noteContainer") {
        axcli::press("Escape")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    let point = scroll_to_note(results, index)?;
    axcli::human_click_point(point)?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    Ok(())
}

// --- show-note ---

pub fn show_note(results: &mut Vec<SearchResult>, index: usize, format: OutputFormat) -> Result<()> {
    open_note_modal(results, index)?;
    extract_and_print_detail(None, format)?;
    axcli::press("Escape")?;
    Ok(())
}

pub fn extract_and_print_detail(
    url_override: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    let mut detail = parser::NoteDetail::default();

    detail.url = url_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| axcli::get_active_tab_url().unwrap_or_default());

    if let Some(node) = axcli::locate_opt("#detail-title") {
        detail.title = node.text(4).trim().to_string();
    }
    if let Some(node) = axcli::locate_opt(".interaction-container >> link >> nth=1") {
        detail.author = node.text(1).trim().to_string();
    }

    if let Some(desc) = axcli::locate_opt("#detail-desc") {
        let tag_nodes = desc.locate_all("#hash-tag");
        for tag_node in &tag_nodes {
            let text = tag_node.text(1).trim().to_string();
            if !text.is_empty() && !detail.tags.contains(&text) {
                detail.tags.push(text);
            }
        }
        let full_text = desc.text(8);
        let mut content = full_text;
        for tag in &detail.tags {
            content = content.replace(tag, "");
        }
        detail.content = content.trim().to_string();

        if detail.tags.is_empty() {
            for tag in extract_inline_tags(&detail.content) {
                if !detail.tags.contains(&tag) {
                    detail.tags.push(tag);
                }
            }
        }
    }

    if let Some(scroller) = axcli::locate_opt(".note-scroller") {
        let texts = scroller.texts(8);
        for text in texts {
            let t = text.trim();
            if t.contains('-') && t.chars().any(|c| c.is_ascii_digit()) && t.len() <= 20 {
                detail.date = t.to_string();
                break;
            }
        }
    }

    if let Some(node) = axcli::locate_opt(".engage-bar-container >> .like-wrapper") {
        detail.likes = node.text(4).trim().to_string();
    }
    detail.liked = axcli::detect_element_color(".engage-bar >> .like-wrapper >> .like-icon")
        .map(|c| c == DominantColor::Red)
        .unwrap_or(false);

    if let Some(node) = axcli::locate_opt(".engage-bar-container >> .collect-wrapper")
        .or_else(|| axcli::locate_opt("#note-page-collect-board-guide"))
    {
        detail.favorites = node.text(4).trim().to_string();
    }
    let collect_icon = if axcli::exists("#note-page-collect-board-guide") {
        "#note-page-collect-board-guide >> .collect-icon"
    } else {
        ".engage-bar >> .collect-wrapper >> .collect-icon"
    };
    detail.favorited = axcli::detect_element_color(collect_icon)
        .map(|c| c == DominantColor::Yellow)
        .unwrap_or(false);

    if let Some(node) = axcli::locate_opt(".engage-bar-container >> .chat-wrapper") {
        detail.comments_count = node.text(4).trim().to_string();
    }
    if let Some(node) = axcli::locate_opt(".comments-el >> .total") {
        detail.total_comments = node.text(2).trim().to_string();
    }

    output::print_note_detail(&detail, format);
    Ok(())
}

fn extract_inline_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for part in content.split('#').skip(1) {
        let tag = part.split_whitespace().next().unwrap_or("").trim();
        if !tag.is_empty() {
            tags.push(format!("#{}", tag));
        }
    }
    tags
}

// --- show-user ---

fn show_user(
    session: &mut Session,
    results: &mut Vec<SearchResult>,
    index: usize,
    size: usize,
    format: OutputFormat,
) -> Result<()> {
    validate_index(results, index)?;

    // Check if we already have a child tab for this user
    let author = &results[index].author;
    if let Some(pos) = session.child_tabs.iter().position(|t| t.nickname == *author) {
        return show_existing_user(session, pos, size, format);
    }

    let main_tab_id = session.tab_id.clone();

    // Open note modal, then click author avatar
    let note_point = scroll_to_note(results, index)?;
    axcli::human_click_point(note_point)?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    axcli::human_click(".interaction-container >> .avatar-click")?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let child_tab_id = axcli::get_active_tab_id()?;
    let is_new_tab = Some(&child_tab_id) != main_tab_id.as_ref();

    for _ in 0..3 {
        if axcli::exists("#userPageContainer") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    if let Some(container) = axcli::locate_opt("#userPageContainer") {
        let profile = parser::extract_user_profile(&container);
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
    } else {
        eprintln!("警告: 无法加载用户主页");
    }

    session.activate_tab()?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    if axcli::exists("#noteContainer") {
        axcli::press("Escape")?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    session.save()?;
    Ok(())
}

fn show_existing_user(
    session: &mut Session,
    child_idx: usize,
    size: usize,
    format: OutputFormat,
) -> Result<()> {
    let child = &session.child_tabs[child_idx];

    if child.results.len() >= size {
        let end = std::cmp::min(size, child.results.len());
        let slice = &child.results[..end];
        let profile = parser::UserProfile {
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
        .map(|c| parser::extract_user_profile(&c))
        .unwrap_or_else(|| parser::UserProfile {
            nickname: session.child_tabs[child_idx].nickname.clone(),
            xhs_id: session.child_tabs[child_idx].xhs_id.clone(),
            ..Default::default()
        });

    let existing = &session.child_tabs[child_idx].results;
    let notes = scroll_and_collect_user_notes(existing, size)?;

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

pub fn scroll_and_collect_user_notes_pub(
    existing: &[SearchResult],
    target_count: usize,
) -> Result<Vec<SearchResult>> {
    scroll_and_collect_user_notes(existing, target_count)
}

fn scroll_and_collect_user_notes(
    existing: &[SearchResult],
    target_count: usize,
) -> Result<Vec<SearchResult>> {
    use std::collections::HashSet;

    let mut seen: HashSet<(String, String)> = existing
        .iter()
        .map(|r| (r.title.clone(), r.author.clone()))
        .collect();
    let mut all: Vec<SearchResult> = existing.to_vec();
    let mut stall = 0;

    for _ in 0..50 {
        axcli::scroll_element_down("#userPageContainer", 500)?;
        std::thread::sleep(std::time::Duration::from_secs(2));

        if let Some(container) = axcli::locate_opt("#userPageContainer") {
            let visible = parser::extract_note_cards(&container);
            let mut new_count = 0;
            for card in &visible {
                let key = (card.title.clone(), card.author.clone());
                if seen.insert(key) {
                    all.push(SearchResult {
                        index: all.len(),
                        title: card.title.clone(),
                        author: card.author.clone(),
                        likes: card.likes.clone(),
                    });
                    new_count += 1;
                }
            }
            if all.len() >= target_count {
                break;
            }
            if new_count == 0 {
                stall += 1;
                if stall >= 3 {
                    break;
                }
            } else {
                stall = 0;
            }
        } else {
            break;
        }
    }
    Ok(all)
}

// --- like/favorite ---

fn is_liked() -> bool {
    axcli::detect_element_color(".engage-bar >> .like-wrapper >> .like-icon")
        .map(|c| c == DominantColor::Red)
        .unwrap_or(false)
}

fn is_favorited() -> bool {
    let selector = if axcli::exists("#note-page-collect-board-guide") {
        "#note-page-collect-board-guide >> .collect-icon"
    } else {
        ".engage-bar >> .collect-wrapper >> .collect-icon"
    };
    axcli::detect_element_color(selector)
        .map(|c| c == DominantColor::Yellow)
        .unwrap_or(false)
}

fn like_note(results: &mut Vec<SearchResult>, index: usize, unlike: bool, format: OutputFormat) -> Result<()> {
    open_note_modal(results, index)?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    let already_liked = is_liked();

    if unlike {
        if !already_liked {
            output::print_action_result("unlike", false, "当前未点赞，无需取消", format);
        } else {
            axcli::human_click(".engage-bar >> .like-wrapper")?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            output::print_action_result("unlike", true, "已取消点赞", format);
        }
    } else if already_liked {
        output::print_action_result("like", false, "已经点赞过了", format);
    } else {
        axcli::human_click(".engage-bar >> .like-wrapper")?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        output::print_action_result("like", true, "点赞成功", format);
    }

    let _ = axcli::press("Escape");
    Ok(())
}

fn favorite_note(results: &mut Vec<SearchResult>, index: usize, unfavorite: bool, format: OutputFormat) -> Result<()> {
    open_note_modal(results, index)?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    let collect_selector = if axcli::exists("#note-page-collect-board-guide") {
        "#note-page-collect-board-guide"
    } else {
        ".engage-bar >> .collect-wrapper"
    };
    let already_favorited = is_favorited();

    if unfavorite {
        if !already_favorited {
            output::print_action_result("unfavorite", false, "当前未收藏，无需取消", format);
        } else {
            axcli::human_click(collect_selector)?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            output::print_action_result("unfavorite", true, "已取消收藏", format);
        }
    } else if already_favorited {
        output::print_action_result("favorite", false, "已经收藏过了", format);
    } else {
        axcli::human_click(collect_selector)?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        output::print_action_result("favorite", true, "收藏成功", format);
    }

    let _ = axcli::press("Escape");
    Ok(())
}

// --- comment ---

fn comment_note(results: &mut Vec<SearchResult>, index: usize, content: &str, format: OutputFormat) -> Result<()> {
    open_note_modal(results, index)?;

    axcli::human_click("textarea#content-textarea")?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    axcli::input("textarea#content-textarea", content)?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    axcli::human_click(".submit")?;
    std::thread::sleep(std::time::Duration::from_secs(1));

    output::print_action_result("comment", true, &format!("评论已发送: {}", content), format);
    let _ = axcli::press("Escape");
    Ok(())
}

// --- show-comments ---

fn show_comments(
    results: &mut Vec<SearchResult>,
    index: usize,
    size: usize,
    format: OutputFormat,
) -> Result<()> {
    open_note_modal(results, index)?;

    let comments = scroll_and_collect_comments(size)?;
    let total = axcli::locate_opt(".comments-el >> .total")
        .map(|n| n.text(2).trim().to_string())
        .unwrap_or_default();

    let end = std::cmp::min(size, comments.len());
    let display = &comments[..end];
    output::print_comments(display, format, &total);

    axcli::press("Escape")?;
    Ok(())
}

fn scroll_and_collect_comments(target_count: usize) -> Result<Vec<parser::Comment>> {
    let mut all_comments = Vec::new();
    let mut stall = 0;

    for _ in 0..50 {
        if let Some(container) = axcli::locate_opt(".comments-el") {
            let visible = parser::extract_comments(&container);
            if visible.len() > all_comments.len() {
                all_comments = visible;
                stall = 0;
            }
        }
        if all_comments.len() >= target_count {
            break;
        }

        // Scroll the note-scroller to bottom of comments to trigger lazy loading.
        // Calculate how far to scroll: use the comments-el height (which can be
        // much larger than the viewport) to jump past already-loaded comments.
        let scroll_amount = axcli::locate_opt(".comments-el")
            .and_then(|n| n.size().map(|(_, h)| (h as i32).max(500)))
            .unwrap_or(500);

        if axcli::exists(".note-scroller") {
            axcli::scroll_element_down(".note-scroller", scroll_amount)?;
            std::thread::sleep(std::time::Duration::from_secs(2));
        } else {
            break;
        }
        // Check for new comments after scroll
        let new_count = axcli::locate_opt(".comments-el")
            .map(|c| parser::extract_comments(&c).len())
            .unwrap_or(0);
        if new_count <= all_comments.len() {
            stall += 1;
            if stall >= 3 {
                break;
            }
        }
    }
    Ok(all_comments)
}
