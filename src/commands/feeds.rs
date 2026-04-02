use anyhow::Result;
use std::collections::HashSet;

use crate::output::OutputFormat;
use crate::session::SearchResult;
use crate::{axcli, output, parser, session};
use super::actions::{NoteAction, dispatch_note_action};
use super::search::scroll_and_collect;

pub fn feeds(
    session_id: Option<&str>,
    size: usize,
    action: Option<NoteAction>,
    format: OutputFormat,
) -> Result<()> {
    let mut session = session::get_active_session(session_id)?;

    // Ensure we're on the feeds page
    ensure_feeds_page(&mut session, size)?;

    let Some(action) = action else {
        let end = std::cmp::min(size, session.results.len());
        let slice = &session.results[..end];
        output::print_list(slice, format, "推荐", &session.id);
        return Ok(());
    };

    let mut results = std::mem::take(&mut session.results);
    let result = dispatch_note_action(&action, &mut session, &mut results, format);
    session.results = results;
    session.save()?;
    result
}

fn ensure_feeds_page(
    session: &mut session::Session,
    size: usize,
) -> Result<()> {
    let already_on_home = session.page_type == session::PageType::Home;

    if already_on_home && session.results.len() >= size {
        return Ok(());
    }

    if already_on_home {
        let notes = scroll_and_collect_feeds(&session.results, size)?;
        session.results = notes;
        session.save()?;
        return Ok(());
    }

    // Navigate to homepage via logo
    axcli::human_click("link#link-guide")?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    if !axcli::exists("#exploreFeeds") {
        axcli::navigate_open("https://www.xiaohongshu.com/explore")?;
    }

    let container_locator = if axcli::exists("#exploreFeeds") {
        "#exploreFeeds"
    } else {
        ".feeds-container"
    };

    let cards = scroll_and_collect(container_locator, size, 50)?;

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
    session.page_type = session::PageType::Home;
    session.clear_scene_params();
    session.save()?;
    Ok(())
}

fn scroll_and_collect_feeds(
    existing: &[SearchResult],
    target_count: usize,
) -> Result<Vec<SearchResult>> {
    let container_locator = if axcli::exists("#exploreFeeds") {
        "#exploreFeeds"
    } else {
        ".feeds-container"
    };

    let mut seen: HashSet<(String, String)> = existing
        .iter()
        .map(|r| (r.title.clone(), r.author.clone()))
        .collect();
    let mut all: Vec<SearchResult> = existing.to_vec();
    let mut stall = 0;

    for _ in 0..50 {
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
