/// Extract structured data from AXNode trees.

use axcli_lib::accessibility::AXNode;

#[derive(Debug, Clone, serde::Serialize)]
pub struct NoteCard {
    pub title: String,
    pub author: String,
    pub likes: String,
}

/// Extract note cards from a container node (.feeds-container or #exploreFeeds)
pub fn extract_note_cards(container: &AXNode) -> Vec<NoteCard> {
    let card_nodes = container.locate_all(".note-item");
    let mut cards = Vec::new();

    for card in &card_nodes {
        // Title from .title link
        let title = card
            .locate(".title")
            .map(|n| first_text(&n))
            .unwrap_or_default();

        // Author: prefer .name group (no date), fall back to .author link
        let author = card
            .locate(".name")
            .map(|n| first_text(&n))
            .or_else(|| card.locate(".author").map(|n| first_text(&n)))
            .unwrap_or_default();

        // Likes from .like-wrapper
        let likes = card
            .locate(".like-wrapper")
            .map(|n| first_text(&n))
            .unwrap_or_else(|| "0".to_string());

        if !title.is_empty() || !author.is_empty() {
            cards.push(NoteCard {
                title,
                author,
                likes,
            });
        }
    }

    cards
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UserCard {
    pub name: String,
    pub xhs_id: String,
    pub description: String,
    pub followers: String,
    pub notes_count: String,
    pub url: String,
}

/// Extract URL from an AXLink node's AXURL attribute.
fn link_url(node: &AXNode) -> String {
    use axcli_lib::accessibility::attr_value;
    use objc2_core_foundation::CFURL;

    let Some(value) = attr_value(&node.0, "AXURL") else {
        return String::new();
    };
    let url = unsafe { &*(value.as_ref() as *const objc2_core_foundation::CFType as *const CFURL) };
    url.string().to_string()
}

/// Extract user cards from the search results page.
///
/// User cards in XHS search are `.user-info` elements within `.layout`.
/// Each contains a combined text like:
/// "编程猫16小时前更新小红书号：94745206473线上教育粉丝・37.7万笔记・507"
///
/// Structure:
/// ```text
/// .layout
///   group
///     link
///       .avatar-container
///       .user-info  "名字...小红书号：xxx...粉丝・N笔记・N"
///         statictext (multiple)
///       .btn
/// ```
pub fn extract_user_cards(container: &AXNode) -> Vec<UserCard> {
    let card_nodes = container.locate_all(".user-info");
    let mut cards = Vec::new();

    for card in &card_nodes {
        let full_text = card.text(8);
        if let Some(mut parsed) = parse_user_info_text(&full_text) {
            // Extract URL from parent link element's AXURL attribute
            if let Some(parent) = card.parent() {
                if parent.role().as_deref() == Some("AXLink") {
                    parsed.url = link_url(&parent);
                }
            }
            cards.push(parsed);
        }
    }

    cards
}

/// Parse a user-info combined text into a UserCard.
///
/// Input examples:
///   "编程猫16小时前更新小红书号：94745206473线上教育粉丝・37.7万笔记・507"
///   "波哥聊编程小红书号：bglbc666粉丝・5.5万笔记・213"
pub fn parse_user_info_text(text: &str) -> Option<UserCard> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // Split by "小红书号：" or "小红书号:"
    let (before_id, after_id) = if let Some(pos) = text.find("小红书号：") {
        (&text[..pos], &text[pos + "小红书号：".len()..])
    } else if let Some(pos) = text.find("小红书号:") {
        (&text[..pos], &text[pos + "小红书号:".len()..])
    } else {
        return None;
    };

    // Extract name from before_id: strip trailing update time like "16小时前更新", "5天前更新"
    let name = strip_update_time(before_id);
    if name.is_empty() {
        return None;
    }

    // Split after_id by "粉丝" to get xhs_id and the rest
    let (xhs_id, after_followers_label) = if let Some(pos) = after_id.find("粉丝") {
        (after_id[..pos].trim().to_string(), &after_id[pos + "粉丝".len()..])
    } else {
        (after_id.trim().to_string(), "")
    };

    // Extract followers count (after "・" or ":" up to "笔记")
    let (followers, after_notes_label) = if let Some(pos) = after_followers_label.find("笔记") {
        let f = after_followers_label[..pos].trim_start_matches('・').trim().to_string();
        (f, &after_followers_label[pos + "笔记".len()..])
    } else {
        let f = after_followers_label.trim_start_matches('・').trim().to_string();
        (f, "")
    };

    // Notes count (after "・")
    let notes_count = after_notes_label.trim_start_matches('・').trim().to_string();

    // Description: any text between xhs_id and "粉丝" that isn't purely the ID
    // The xhs_id field may contain trailing description text
    // e.g., "94745206473线上教育" → id="94745206473", desc="线上教育"
    let (clean_id, description) = split_id_and_desc(&xhs_id);

    Some(UserCard {
        name,
        xhs_id: clean_id,
        description,
        followers,
        notes_count,
        url: String::new(),
    })
}

/// Strip trailing update time patterns like "16小时前更新", "5天前更新", "3分钟前更新"
fn strip_update_time(s: &str) -> String {
    // Pattern: digits + (分钟|小时|天) + "前更新"
    if let Some(pos) = s.find("前更新") {
        let before = &s[..pos];
        for unit in &["分钟", "小时", "天"] {
            if let Some(unit_pos) = before.rfind(unit) {
                let prefix = &before[..unit_pos];
                if prefix.ends_with(|c: char| c.is_ascii_digit()) {
                    // Walk char_indices to find where trailing digits start
                    let chars: Vec<(usize, char)> = prefix.char_indices().collect();
                    let mut name_end_byte = 0;
                    for i in (0..chars.len()).rev() {
                        if !chars[i].1.is_ascii_digit() {
                            name_end_byte = chars[i].0 + chars[i].1.len_utf8();
                            break;
                        }
                    }
                    return s[..name_end_byte].trim().to_string();
                }
            }
        }
    }
    s.trim().to_string()
}

/// Split a combined xhs_id + description string.
/// The ID is numeric or alphanumeric, description follows.
/// e.g., "94745206473线上教育" → ("94745206473", "线上教育")
/// e.g., "codinghou" → ("codinghou", "")
fn split_id_and_desc(s: &str) -> (String, String) {
    // If it's all ASCII alphanumeric, it's just the ID
    if s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return (s.to_string(), String::new());
    }

    // Find where non-ID characters start (first CJK or non-alnum char after initial ID)
    let mut id_end = 0;
    for (i, c) in s.char_indices() {
        if c.is_ascii_alphanumeric() || c == '_' {
            id_end = i + c.len_utf8();
        } else {
            break;
        }
    }

    if id_end == 0 {
        return (String::new(), s.to_string());
    }

    let id = s[..id_end].to_string();
    let desc = s[id_end..].trim().to_string();
    (id, desc)
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct UserProfile {
    pub nickname: String,
    pub xhs_id: String,
    pub ip_location: String,
    pub description: String,
    pub following: String,
    pub followers: String,
    pub likes_and_favorites: String,
}

/// Extract user profile from #userPageContainer node
pub fn extract_user_profile(container: &AXNode) -> UserProfile {
    let mut profile = UserProfile::default();

    // Nickname
    if let Some(name_node) = container.locate(".user-name") {
        profile.nickname = first_text(&name_node);
    }

    // Description
    if let Some(desc_node) = container.locate(".user-desc") {
        profile.description = first_text(&desc_node);
    }

    // XHS ID, IP location, and stats from text nodes
    let texts = container.texts(10);
    for (i, text) in texts.iter().enumerate() {
        let t = text.trim();
        if let Some(id) = t.strip_prefix("小红书号：").or_else(|| t.strip_prefix("小红书号:")) {
            profile.xhs_id = id.trim().to_string();
        }
        if let Some(loc) = t.strip_prefix("IP属地：").or_else(|| t.strip_prefix("IP属地:")) {
            profile.ip_location = loc.trim().to_string();
        }
        // Stats: number appears before label
        if t == "关注" && i > 0 {
            profile.following = texts[i - 1].trim().to_string();
        } else if t == "粉丝" && i > 0 {
            profile.followers = texts[i - 1].trim().to_string();
        } else if t == "获赞与收藏" && i > 0 {
            profile.likes_and_favorites = texts[i - 1].trim().to_string();
        }
    }

    profile
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct NoteDetail {
    pub title: String,
    pub author: String,
    pub url: String,
    pub content: String,
    pub tags: Vec<String>,
    pub date: String,
    pub likes: String,
    pub liked: bool,
    pub favorites: String,
    pub favorited: bool,
    pub comments_count: String,
    pub total_comments: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct NotificationItem {
    pub user: String,
    pub action: String,
    pub time: String,
    pub content: String,
}

/// Extract notification items from the notification page.
///
/// Notification items are flat siblings in the DOM (not wrapped in containers):
/// ```text
/// .layout
///   link.user-avatar      ← start of notification 1
///   link "用户名1"
///   .interaction-hint     ← "评论了你的笔记33分钟前"
///   .interaction-content  ← "评论内容"
///   .action-reply
///   .like-wrapper
///   .extra
///   link.user-avatar      ← start of notification 2
///   link "用户名2"
///   .interaction-hint
///   ...
/// ```
///
/// We iterate all children of the container sequentially and group them
/// by using `link.user-avatar` as the boundary of each notification.
pub fn extract_notifications(container: &AXNode) -> Vec<NotificationItem> {
    let children = container.children();
    let mut items: Vec<NotificationItem> = Vec::new();
    let mut current: Option<NotificationItem> = None;

    for child in &children {
        let classes = child.dom_classes();

        if classes.contains(&"user-avatar".to_string()) {
            // New notification starts — save the previous one
            if let Some(item) = current.take() {
                if !item.action.is_empty() {
                    items.push(item);
                }
            }
            current = Some(NotificationItem::default());
            continue;
        }

        let Some(ref mut item) = current else {
            continue;
        };

        if classes.contains(&"interaction-hint".to_string()) {
            let texts = child.texts(4);
            if texts.len() >= 2 {
                item.action = texts[0].trim().to_string();
                item.time = texts[1].trim().to_string();
            } else {
                item.action = first_text(child);
            }
        } else if classes.contains(&"interaction-content".to_string()) {
            item.content = first_text(child);
        } else if child.role().as_deref() == Some("AXLink")
            && !classes.contains(&"link-wrapper".to_string())
            && !classes.contains(&"icp-text".to_string())
            && !classes.contains(&"active".to_string())
        {
            // Username link (non-nav, non-avatar link with text)
            let text = child.text(1).trim().to_string();
            if !text.is_empty() && text.chars().count() < 50 {
                item.user = text;
            }
        }
    }

    // Don't forget the last notification
    if let Some(item) = current {
        if !item.action.is_empty() {
            items.push(item);
        }
    }

    items
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Comment {
    pub author: String,
    pub content: String,
    pub likes: String,
    pub date: String,
}

/// Extract top-level comments from the comments section of a note detail.
///
/// Structure (verified via inspect):
/// ```text
/// .comments-el
///   .list-container
///     .comment-item              ← top-level comment
///       .comment-inner-container
///         link.name   "作者名"
///         .content    "评论内容"
///         .date       "03-16中国香港"
///         .like-wrapper "4"
///     .list-container            ← nested replies (skip)
///     .comment-item              ← next top-level comment
/// ```
pub fn extract_comments(container: &AXNode) -> Vec<Comment> {
    // Get the top-level .list-container inside .comments-el
    let list = match container.locate(".list-container") {
        Some(l) => l,
        None => return vec![],
    };

    // Only get direct .comment-item children of the top-level list
    // (not nested ones inside sub .list-container)
    let comment_nodes = list.locate_all(".comment-item");
    let mut comments = Vec::new();

    for node in &comment_nodes {
        // Look inside .comment-inner-container for the data fields
        let author = node
            .locate(".comment-inner-container >> .name")
            .or_else(|| node.locate(".name"))
            .map(|n| first_text(&n))
            .unwrap_or_default();
        let content = node
            .locate(".comment-inner-container >> .content")
            .or_else(|| node.locate(".content"))
            .map(|n| first_text(&n))
            .unwrap_or_default();
        let likes = node
            .locate(".comment-inner-container >> .like-wrapper")
            .or_else(|| node.locate(".like-wrapper"))
            .map(|n| first_text(&n))
            .unwrap_or_else(|| "0".to_string());
        let date = node
            .locate(".comment-inner-container >> .date")
            .or_else(|| node.locate(".date"))
            .map(|n| first_text(&n))
            .unwrap_or_default();

        if !content.is_empty() {
            comments.push(Comment {
                author,
                content,
                likes,
                date,
            });
        }
    }
    comments
}

/// Get the first non-empty text from a node's subtree
fn first_text(node: &AXNode) -> String {
    let texts = node.texts(4);
    texts
        .into_iter()
        .find(|t| !t.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .to_string()
}
