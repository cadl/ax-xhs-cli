use anyhow::{Context, Result, bail};
use std::collections::HashSet;

use crate::output::OutputFormat;
use crate::session::SearchResult;
use crate::{axcli, output, parser, session};
use super::actions::{NoteAction, dispatch_note_action};

pub fn search(
    session_id: Option<&str>,
    keyword: Option<&str>,
    sort: Option<&str>,
    note_type: Option<&str>,
    time: Option<&str>,
    scope: Option<&str>,
    location: Option<&str>,
    size: usize,
    action: Option<NoteAction>,
    format: OutputFormat,
) -> Result<()> {
    let mut session = session::get_active_session(session_id)?;

    // Resolve keyword: use passed value, or fall back to saved scene param
    let keyword = keyword
        .or_else(|| session.scene_param("keyword"))
        .ok_or_else(|| anyhow::anyhow!("请指定 --scene-keyword 参数"))?
        .to_string();

    // Build scene params from passed values
    let new_params: Vec<(&str, Option<&str>)> = vec![
        ("keyword", Some(keyword.as_str())),
        ("sort", sort),
        ("note_type", note_type),
        ("time", time),
        ("scope", scope),
        ("location", location),
    ];

    // Check scene param consistency only when executing a subcommand
    if action.is_some() && session.page_type == session::PageType::Search && !session.scene_params.is_empty() {
        let mismatches = session.check_scene_params(&new_params);
        if !mismatches.is_empty() {
            eprintln!("错误: 场景参数不一致，当前 session 场景参数:");
            for (key, saved, new) in &mismatches {
                eprintln!("  {} = \"{}\"（命令指定: \"{}\"）", key, saved, new);
            }
            eprintln!("\n如需切换场景，请先不带子命令执行 search 进入新场景");
            bail!("场景参数不一致");
        }
    }

    // Determine if we need to search
    // None = not specified → keep saved value (no re-search)
    // Some(v) where v differs from saved → re-search
    let param_changed = |key: &str, new_val: Option<&str>| -> bool {
        match new_val {
            None => false,
            Some(v) => session.scene_param(key) != Some(v),
        }
    };
    let need_search = session.page_type != session::PageType::Search
        || session.scene_param("keyword") != Some(keyword.as_str())
        || param_changed("sort", sort)
        || param_changed("note_type", note_type)
        || param_changed("time", time)
        || param_changed("scope", scope)
        || param_changed("location", location);

    if need_search {
        // Enter new search scene
        do_search(&mut session, &keyword, sort, note_type, time, scope, location, size)?;
    } else if session.results.len() < size {
        // Same scene, need more results
        let cards = scroll_and_collect_with_existing(
            ".feeds-container",
            &session.results,
            size,
            50,
        )?;
        session.results = cards;
        session.save()?;
    }

    // If no action, list results
    let Some(action) = action else {
        let end = std::cmp::min(size, session.results.len());
        let slice = &session.results[..end];
        output::print_list(slice, format, "搜索结果", &session.id);
        return Ok(());
    };

    let mut results = std::mem::take(&mut session.results);
    let result = dispatch_note_action(&action, &mut session, &mut results, format);
    session.results = results;
    session.save()?;
    result
}

fn do_search(
    session: &mut session::Session,
    keyword: &str,
    sort: Option<&str>,
    note_type: Option<&str>,
    time: Option<&str>,
    scope: Option<&str>,
    location: Option<&str>,
    size: usize,
) -> Result<()> {
    // Go to homepage first
    if session.page_type != session::PageType::Home {
        axcli::human_click("link#link-guide")?;
        std::thread::sleep(std::time::Duration::from_secs(2));
        if !axcli::exists("#exploreFeeds") {
            axcli::navigate_open("https://www.xiaohongshu.com/explore")?;
        }
    }

    axcli::human_click("textfield#search-input")?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    axcli::focus("textfield#search-input")?;
    std::thread::sleep(std::time::Duration::from_millis(200));
    axcli::input("textfield#search-input", keyword)?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    axcli::press("Enter")?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let has_filters = sort.is_some()
        || note_type.is_some()
        || time.is_some()
        || scope.is_some()
        || location.is_some();
    if has_filters {
        apply_filters(sort, note_type, time, scope, location)?;
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    let cards = scroll_and_collect(".feeds-container", size, 50)?;

    // Save scene params
    session.clear_scene_params();
    session.set_scene_param("keyword", keyword);
    if let Some(v) = sort { session.set_scene_param("sort", v); }
    if let Some(v) = note_type { session.set_scene_param("note_type", v); }
    if let Some(v) = time { session.set_scene_param("time", v); }
    if let Some(v) = scope { session.set_scene_param("scope", v); }
    if let Some(v) = location { session.set_scene_param("location", v); }

    session.results = cards
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
    session.page_type = session::PageType::Search;
    session.save()?;
    Ok(())
}

fn scroll_and_collect_with_existing(
    container_locator: &str,
    existing: &[SearchResult],
    target_count: usize,
    max_scrolls: usize,
) -> Result<Vec<SearchResult>> {
    let mut seen: HashSet<(String, String)> = existing
        .iter()
        .map(|r| (r.title.clone(), r.author.clone()))
        .collect();
    let mut all: Vec<SearchResult> = existing.to_vec();
    let mut stall = 0;

    for _ in 0..max_scrolls {
        axcli::scroll_element_down(container_locator, 500)?;
        std::thread::sleep(std::time::Duration::from_secs(2));

        let container = axcli::locate(container_locator)?;
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
                    url: String::new(),
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

/// Scroll and collect note cards from a container, deduplicating by (title, author).
pub fn scroll_and_collect(
    container_locator: &str,
    target_count: usize,
    max_scrolls: usize,
) -> Result<Vec<parser::NoteCard>> {
    let mut all_cards: Vec<parser::NoteCard> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut stall = 0;

    for _ in 0..=max_scrolls {
        let container = axcli::locate(container_locator)?;
        let visible = parser::extract_note_cards(&container);

        let mut new_count = 0;
        for card in &visible {
            let key = (card.title.clone(), card.author.clone());
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

fn apply_filters(
    sort: Option<&str>,
    note_type: Option<&str>,
    time: Option<&str>,
    scope: Option<&str>,
    location: Option<&str>,
) -> Result<()> {
    axcli::hover(".filter")?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    if !axcli::exists(".filter-panel") {
        axcli::hover(".filter")?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let filters: Vec<Option<&str>> = vec![sort, note_type, time, scope, location];
    let defaults = ["综合", "不限", "不限", "不限", "不限"];

    for (filter, default) in filters.iter().zip(defaults.iter()) {
        if let Some(value) = filter {
            if *value != *default {
                let selector = format!(".tags:has-text(\"{}\")", value);
                axcli::human_click(&selector).context(format!("无法选择筛选项: {}", value))?;
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        }
    }
    Ok(())
}
