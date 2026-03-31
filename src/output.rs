use serde::Serialize;

use crate::parser::{Comment, NoteDetail, NotificationItem, UserProfile};
use crate::session::SearchResult;

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Yaml,
}

/// Print any serializable value as JSON or YAML
pub fn print_value<T: Serialize>(value: &T, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value).unwrap());
        }
        OutputFormat::Yaml => {
            print!("{}", serde_yaml::to_string(value).unwrap());
        }
        OutputFormat::Text => {
            // Fallback to JSON for types without custom text formatting
            println!("{}", serde_json::to_string_pretty(value).unwrap());
        }
    }
}

/// Print a list of search/feed results
pub fn print_list(
    items: &[SearchResult],
    format: OutputFormat,
    summary_label: &str,
    session_id: &str,
) {
    match format {
        OutputFormat::Text => {
            if items.is_empty() {
                println!("没有{}", summary_label);
            } else {
                for r in items {
                    println!("[{}] {} - {} ❤{}", r.index, r.title, r.author, r.likes);
                }
                println!(
                    "\n共 {} 条{} (session: {})",
                    items.len(),
                    summary_label,
                    session_id
                );
            }
        }
        _ => print_value(&items, format),
    }
}

/// Print note detail
pub fn print_note_detail(detail: &NoteDetail, format: OutputFormat) {
    match format {
        OutputFormat::Text => {
            println!("标题: {}", detail.title);
            println!("作者: {}", detail.author);
            println!("链接: {}", detail.url);
            println!("日期: {}", detail.date);
            println!();
            println!("{}", detail.content);
            if !detail.tags.is_empty() {
                println!("\n标签: {}", detail.tags.join(" "));
            }
            println!();
            let liked_mark = if detail.liked { " ✓" } else { "" };
            let fav_mark = if detail.favorited { " ✓" } else { "" };
            println!(
                "❤{}{} ⭐{}{} 💬{}",
                detail.likes, liked_mark, detail.favorites, fav_mark, detail.comments_count
            );
            if !detail.total_comments.is_empty() {
                println!("评论总数: {}", detail.total_comments);
            }
        }
        _ => print_value(detail, format),
    }
}

/// Print user profile with notes
pub fn print_user(profile: &UserProfile, notes: &[SearchResult], format: OutputFormat) {
    match format {
        OutputFormat::Text => {
            println!("昵称: {}", profile.nickname);
            if !profile.xhs_id.is_empty() {
                println!("小红书号: {}", profile.xhs_id);
            }
            if !profile.ip_location.is_empty() {
                println!("IP属地: {}", profile.ip_location);
            }
            if !profile.description.is_empty() {
                println!("简介: {}", profile.description);
            }
            println!(
                "关注 {} | 粉丝 {} | 获赞与收藏 {}",
                profile.following, profile.followers, profile.likes_and_favorites
            );
            if !notes.is_empty() {
                println!("\n笔记:");
                for r in notes {
                    println!("  [{}] {} - ❤{}", r.index, r.title, r.likes);
                }
                println!("\n共 {} 条笔记", notes.len());
            }
        }
        _ => {
            #[derive(Serialize)]
            struct UserOutput<'a> {
                profile: &'a UserProfile,
                notes: &'a [SearchResult],
            }
            print_value(&UserOutput { profile, notes }, format);
        }
    }
}

/// Print comments list
pub fn print_comments(comments: &[Comment], format: OutputFormat, total: &str) {
    match format {
        OutputFormat::Text => {
            if comments.is_empty() {
                println!("没有评论");
            } else {
                for (i, c) in comments.iter().enumerate() {
                    println!("[{}] {} - {}", i, c.author, c.content);
                    if !c.likes.is_empty() && c.likes != "0" {
                        print!("    ❤{}", c.likes);
                    }
                    if !c.date.is_empty() {
                        print!("  {}", c.date);
                    }
                    if !c.likes.is_empty() || !c.date.is_empty() {
                        println!();
                    }
                }
                if !total.is_empty() {
                    println!("\n评论总数: {}（含回复）", total);
                }
            }
        }
        _ => {
            #[derive(Serialize)]
            struct CommentsOutput<'a> {
                comments: &'a [Comment],
                total: &'a str,
            }
            print_value(&CommentsOutput { comments, total }, format);
        }
    }
}

/// Print notification items
pub fn print_notifications(items: &[NotificationItem], format: OutputFormat) {
    match format {
        OutputFormat::Text => {
            if items.is_empty() {
                println!("没有通知");
            } else {
                for (i, item) in items.iter().enumerate() {
                    if !item.user.is_empty() {
                        print!("[{}] {} {}", i, item.user, item.action);
                    } else {
                        print!("[{}] {}", i, item.action);
                    }
                    if !item.time.is_empty() {
                        print!("  {}", item.time);
                    }
                    println!();
                    if !item.content.is_empty() {
                        println!("    {}", item.content);
                    }
                }
                println!("\n共 {} 条通知", items.len());
            }
        }
        _ => print_value(&items, format),
    }
}

/// Print action result (like/favorite/comment)
pub fn print_action_result(action: &str, success: bool, message: &str, format: OutputFormat) {
    match format {
        OutputFormat::Text => {
            println!("{}", message);
        }
        _ => {
            #[derive(Serialize)]
            struct ActionResult<'a> {
                action: &'a str,
                success: bool,
                message: &'a str,
            }
            print_value(
                &ActionResult {
                    action,
                    success,
                    message,
                },
                format,
            );
        }
    }
}
