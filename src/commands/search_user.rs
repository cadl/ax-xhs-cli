use anyhow::{bail, Result};
use clap::Subcommand;
use std::collections::HashSet;

use crate::output::OutputFormat;
use crate::session::{ChildTab, SearchResult, UserSearchResult};
use crate::{axcli, output, parser, session};
use super::search::{navigate_to_search, scroll_and_collect};

#[derive(Subcommand, Debug)]
pub enum SearchUserAction {
    /// 查看搜索到的用户主页
    #[command(name = "show-user")]
    ShowUser {
        index: usize,
        #[arg(long, default_value = "20")]
        size: usize,
    },
}

pub fn search_user(
    session_id: Option<&str>,
    keyword: Option<&str>,
    size: usize,
    action: Option<SearchUserAction>,
    format: OutputFormat,
) -> Result<()> {
    let mut session = session::get_active_session(session_id)?;

    // Resolve keyword: use passed value, or fall back to saved scene param
    let keyword = keyword
        .or_else(|| session.scene_param("keyword"))
        .ok_or_else(|| anyhow::anyhow!("请指定 --scene-keyword 参数"))?
        .to_string();

    // Check scene param consistency when executing a subcommand
    if action.is_some()
        && session.page_type == session::PageType::SearchUser
        && !session.scene_params.is_empty()
    {
        let new_params = vec![("keyword", Some(keyword.as_str()))];
        let mismatches = session.check_scene_params(&new_params);
        if !mismatches.is_empty() {
            eprintln!("错误: 场景参数不一致，当前 session 场景参数:");
            for (key, saved, new) in &mismatches {
                eprintln!("  {} = \"{}\"（命令指定: \"{}\"）", key, saved, new);
            }
            eprintln!("\n如需切换场景，请先不带子命令执行 search-user 进入新场景");
            bail!("场景参数不一致");
        }
    }

    let need_search = session.page_type != session::PageType::SearchUser
        || session.scene_param("keyword") != Some(keyword.as_str());

    if need_search {
        do_search_user(&mut session, &keyword, size)?;
    } else if session.user_results.len() < size {
        // Same scene, need more results
        let results = scroll_and_collect_users_with_existing(
            ".layout",
            &session.user_results,
            size,
            50,
        )?;
        session.user_results = results;
        session.save()?;
    }

    // If no action, list results
    let Some(action) = action else {
        let end = std::cmp::min(size, session.user_results.len());
        let slice = &session.user_results[..end];
        output::print_user_search_list(slice, format, &session.id);
        return Ok(());
    };

    match action {
        SearchUserAction::ShowUser { index, size: user_size } => {
            show_user_from_search(&mut session, index, user_size, format)?;
        }
    }

    session.save()?;
    Ok(())
}

fn do_search_user(
    session: &mut session::Session,
    keyword: &str,
    size: usize,
) -> Result<()> {
    navigate_to_search(session, keyword)?;

    // Click the "用户" tab in the search results
    axcli::human_click(".channel:has-text(\"用户\")")?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    let cards = scroll_and_collect_users(".layout", size, 50)?;

    // Save scene params
    session.clear_scene_params();
    session.set_scene_param("keyword", keyword);

    session.user_results = cards
        .iter()
        .enumerate()
        .map(|(i, card)| UserSearchResult {
            index: i,
            name: card.name.clone(),
            xhs_id: card.xhs_id.clone(),
            description: card.description.clone(),
            followers: card.followers.clone(),
            notes_count: card.notes_count.clone(),
        })
        .collect();
    session.page_type = session::PageType::SearchUser;
    session.save()?;
    Ok(())
}

/// Scroll and collect user cards, deduplicating by (name, xhs_id).
fn scroll_and_collect_users(
    container_locator: &str,
    target_count: usize,
    max_scrolls: usize,
) -> Result<Vec<parser::UserCard>> {
    let mut all_cards: Vec<parser::UserCard> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut stall = 0;

    for _ in 0..=max_scrolls {
        let container = axcli::locate(container_locator)?;
        let visible = parser::extract_user_cards(&container);

        let mut new_count = 0;
        for card in &visible {
            let key = (card.name.clone(), card.xhs_id.clone());
            if seen.insert(key) {
                all_cards.push(card.clone());
                new_count += 1;
            }
        }
        if all_cards.len() >= target_count {
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
        axcli::scroll_element_down(container_locator, 500)?;
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    Ok(all_cards)
}

fn scroll_and_collect_users_with_existing(
    container_locator: &str,
    existing: &[UserSearchResult],
    target_count: usize,
    max_scrolls: usize,
) -> Result<Vec<UserSearchResult>> {
    let mut seen: HashSet<(String, String)> = existing
        .iter()
        .map(|r| (r.name.clone(), r.xhs_id.clone()))
        .collect();
    let mut all: Vec<UserSearchResult> = existing.to_vec();
    let mut stall = 0;

    for _ in 0..max_scrolls {
        axcli::scroll_element_down(container_locator, 500)?;
        std::thread::sleep(std::time::Duration::from_secs(2));

        let container = axcli::locate(container_locator)?;
        let visible = parser::extract_user_cards(&container);

        let mut new_count = 0;
        for card in &visible {
            let key = (card.name.clone(), card.xhs_id.clone());
            if seen.insert(key) {
                all.push(UserSearchResult {
                    index: all.len(),
                    name: card.name.clone(),
                    xhs_id: card.xhs_id.clone(),
                    description: card.description.clone(),
                    followers: card.followers.clone(),
                    notes_count: card.notes_count.clone(),
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
    }
    Ok(all)
}

/// Scroll to a user card in the search results and return its center point.
fn scroll_to_user_card(
    session: &session::Session,
    index: usize,
) -> Result<core_graphics::geometry::CGPoint> {
    if index >= session.user_results.len() {
        bail!(
            "索引超出范围: {}（共 {} 位用户，索引 0-{}）",
            index,
            session.user_results.len(),
            session.user_results.len() - 1
        );
    }

    let target_name = &session.user_results[index].name;
    let container_locator = ".layout";

    for _ in 0..80 {
        let container = axcli::locate(container_locator)?;
        let card_nodes = container.locate_all(".user-info");

        // Find target card by matching the name from the parsed text
        for node in &card_nodes {
            let full_text = node.text(8);
            let card = parser::parse_user_info_text(&full_text);
            let name = card.as_ref().map(|c| c.name.as_str()).unwrap_or("");

            if name == target_name {
                if let (Some((px, py)), Some((sx, sy))) = (node.position(), node.size()) {
                    let center =
                        core_graphics::geometry::CGPoint::new(px + sx / 2.0, py + sy / 2.0);
                    // Scroll into viewport if needed
                    let (_, vp_top, vp_bottom) = axcli::chrome_viewport()?;
                    let vp_center = (vp_top + vp_bottom) / 2.0;
                    let tolerance = (vp_bottom - vp_top) / 4.0;
                    if (center.y - vp_center).abs() > tolerance {
                        let nudge = (center.y - vp_center) as i32;
                        axcli::scroll_on_container(container_locator, nudge)?;
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        // Re-locate after scroll
                        let container2 = axcli::locate(container_locator)?;
                        for n2 in &container2.locate_all(".user-info") {
                            let text2 = n2.text(8);
                            let card2 = parser::parse_user_info_text(&text2);
                            let name2 = card2.as_ref().map(|c| c.name.as_str()).unwrap_or("");
                            if name2 == target_name {
                                if let (Some((px2, py2)), Some((sx2, sy2))) =
                                    (n2.position(), n2.size())
                                {
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

        // Target not visible — scroll down to find it
        axcli::scroll_element_down(container_locator, 500)?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    bail!("无法定位用户 (index {})", index);
}

fn show_user_from_search(
    session: &mut session::Session,
    index: usize,
    size: usize,
    format: OutputFormat,
) -> Result<()> {
    if index >= session.user_results.len() {
        bail!(
            "索引超出范围: {}（共 {} 位用户，索引 0-{}）",
            index,
            session.user_results.len(),
            session.user_results.len() - 1
        );
    }

    // Check if we already have a child tab for this user
    let user_name = &session.user_results[index].name;
    if let Some(pos) = session
        .child_tabs
        .iter()
        .position(|t| t.nickname == *user_name)
    {
        return show_existing_child_tab(session, pos, size, format);
    }

    let main_tab_id = session.tab_id.clone();

    // Click the user card to navigate to their profile
    let point = scroll_to_user_card(session, index)?;
    axcli::human_click_point(point)?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let child_tab_id = axcli::get_active_tab_id()?;
    let is_new_tab = Some(&child_tab_id) != main_tab_id.as_ref();

    // Wait for user profile to load
    for _ in 0..5 {
        if axcli::exists("#userPageContainer") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    if let Some(container) = axcli::locate_opt("#userPageContainer") {
        let profile = parser::extract_user_profile(&container);
        let cards = scroll_and_collect("#userPageContainer", size, 50).unwrap_or_default();
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
    } else {
        eprintln!("警告: 无法加载用户主页");
    }

    // Switch back to main tab
    session.activate_tab()?;
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

fn show_existing_child_tab(
    session: &mut session::Session,
    child_idx: usize,
    size: usize,
    format: OutputFormat,
) -> Result<()> {
    let child = &session.child_tabs[child_idx];

    // If we already have enough cached results, just print them
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

    // Switch to child tab and collect more notes
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
    let notes = crate::commands::actions::scroll_and_collect_user_notes_pub(existing, size)?;

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
