use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::axcli;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PageType {
    Home,
    Search,
    UserProfile,
    NoteDetail,
    Notification,
    NotLoggedIn,
    Error,
    Unknown,
}

impl std::fmt::Display for PageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PageType::Home => write!(f, "首页"),
            PageType::Search => write!(f, "搜索结果"),
            PageType::UserProfile => write!(f, "用户主页"),
            PageType::NoteDetail => write!(f, "笔记详情"),
            PageType::Notification => write!(f, "通知"),
            PageType::NotLoggedIn => write!(f, "未登录"),
            PageType::Error => write!(f, "错误页"),
            PageType::Unknown => write!(f, "未知"),
        }
    }
}

/// An available action in the current session state
pub struct AvailableAction {
    pub command: &'static str,
    pub description: &'static str,
    pub example: &'static str,
}

impl PageType {
    /// Return available actions for this page type given the number of results
    pub fn available_actions(&self, has_results: bool) -> Vec<AvailableAction> {
        let mut actions = Vec::new();

        // Always available
        actions.push(AvailableAction {
            command: "search",
            description: "搜索笔记（会先回到首页）",
            example: "search -k \"关键词\" --sort 最新",
        });
        actions.push(AvailableAction {
            command: "feeds",
            description: "查看首页推荐（会导航到首页）",
            example: "feeds",
        });
        actions.push(AvailableAction {
            command: "notification",
            description: "查看通知",
            example: "notification",
        });
        actions.push(AvailableAction {
            command: "status",
            description: "检查小红书登录状态",
            example: "status",
        });
        actions.push(AvailableAction {
            command: "session status",
            description: "查看当前 session 状态",
            example: "session status",
        });
        actions.push(AvailableAction {
            command: "session end",
            description: "结束 session 并关闭 tab",
            example: "session end",
        });

        // Actions requiring results (subcommands of the current scene)
        if has_results {
            actions.push(AvailableAction {
                command: "show-note <N>",
                description: "查看第 N 条结果的笔记详情",
                example: "search show-note 0",
            });
            actions.push(AvailableAction {
                command: "show-user <N>",
                description: "查看第 N 条结果的作者主页",
                example: "search show-user 0",
            });
            actions.push(AvailableAction {
                command: "like-note <N>",
                description: "点赞第 N 条结果",
                example: "search like-note 0",
            });
            actions.push(AvailableAction {
                command: "favorite-note <N>",
                description: "收藏第 N 条结果",
                example: "search favorite-note 0",
            });
            actions.push(AvailableAction {
                command: "comment-note <N>",
                description: "评论第 N 条结果",
                example: "search comment-note 0 -c \"好文\"",
            });
            actions.push(AvailableAction {
                command: "show-comments <N>",
                description: "查看第 N 条结果的评论",
                example: "search show-comments 0",
            });
        }

        actions
    }

    /// Brief hint about what to do next
    pub fn next_step_hint(&self, has_results: bool) -> &'static str {
        match (self, has_results) {
            (PageType::Home, false) => "提示: 使用 search 搜索或 feeds 获取推荐",
            (PageType::Home, true) => "提示: 可通过索引操作结果，如 feeds show-note 0 / show-user 0",
            (PageType::Search, false) => "提示: 搜索无结果，尝试其他关键词",
            (PageType::Search, true) => "提示: 可通过索引操作搜索结果，如 search show-note 0 / show-user 0",
            (PageType::Notification, _) => "提示: 当前在通知页，使用 feeds 回首页",
            (PageType::Error, _) => "提示: 页面异常，使用 search 或 feeds 重新导航",
            (PageType::NotLoggedIn, _) => "提示: 请先在浏览器中登录小红书",
            _ => "提示: 使用 search 搜索或 feeds 获取推荐",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub index: usize,
    pub title: String,
    pub author: String,
    pub likes: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildTab {
    pub tab_id: String,
    pub nickname: String,
    pub xhs_id: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub page_type: PageType,
    /// Chrome tab ID (stable across navigations)
    pub tab_id: Option<String>,
    /// Scene parameters (saved per-scene context)
    #[serde(default)]
    pub scene_params: std::collections::HashMap<String, String>,
    pub results: Vec<SearchResult>,
    /// Open user profile tabs
    #[serde(default)]
    pub child_tabs: Vec<ChildTab>,
    /// Legacy field for backwards compat with old session files
    #[serde(default, skip_serializing)]
    child_tab_ids: Vec<String>,
}

impl Session {
    pub fn new(name: &str) -> Self {
        Self {
            id: name.to_string(),
            description: name.to_string(),
            created_at: Utc::now(),
            page_type: PageType::Unknown,
            tab_id: None,
            scene_params: std::collections::HashMap::new(),
            results: vec![],
            child_tabs: vec![],
            child_tab_ids: vec![],
        }
    }

    /// Get a scene param value
    pub fn scene_param(&self, key: &str) -> Option<&str> {
        self.scene_params.get(key).map(|s| s.as_str())
    }

    /// Set a scene param
    pub fn set_scene_param(&mut self, key: &str, value: &str) {
        self.scene_params.insert(key.to_string(), value.to_string());
    }

    /// Clear all scene params (when switching scenes)
    pub fn clear_scene_params(&mut self) {
        self.scene_params.clear();
    }

    /// Check if passed scene params match saved ones.
    /// Returns list of mismatched param names, or empty if all match.
    pub fn check_scene_params(&self, params: &[(&str, Option<&str>)]) -> Vec<(String, String, String)> {
        let mut mismatches = Vec::new();
        for (key, new_val) in params {
            if let Some(new_v) = new_val {
                if let Some(saved_v) = self.scene_params.get(*key) {
                    if saved_v != new_v {
                        mismatches.push((
                            key.to_string(),
                            saved_v.clone(),
                            new_v.to_string(),
                        ));
                    }
                }
            }
        }
        mismatches
    }

    /// Activate this session's Chrome tab (switch to it)
    pub fn activate_tab(&self) -> Result<()> {
        if let Some(ref tab_id) = self.tab_id {
            axcli::switch_to_tab(tab_id)?;
        }
        Ok(())
    }

    /// Bind session to the currently active Chrome tab
    pub fn bind_to_current_tab(&mut self) -> Result<()> {
        self.tab_id = Some(axcli::get_active_tab_id()?);
        Ok(())
    }

    /// Detect current page type by examining the accessibility tree
    pub fn detect_page_type(&mut self) -> Result<PageType> {
        let page_type =
            if axcli::exists("#noteContainer") || axcli::exists(".note-detail-mask") {
                PageType::NoteDetail
            } else if axcli::exists("#userPageContainer") {
                PageType::UserProfile
            } else if axcli::exists(".tertiary") {
                PageType::Notification
            } else if axcli::exists("#exploreFeeds") {
                PageType::Home
            } else if axcli::exists(".feeds-container") {
                PageType::Search
            } else {
                PageType::Unknown
            };

        self.page_type = page_type.clone();
        self.save()?;
        Ok(page_type)
    }

    /// Close all child tabs and switch back to the main tab
    pub fn close_all_child_tabs(&mut self) -> Result<()> {
        for child in self.child_tabs.drain(..) {
            let _ = axcli::close_tab_by_id(&child.tab_id);
        }
        // Legacy cleanup
        for tab_id in self.child_tab_ids.drain(..) {
            let _ = axcli::close_tab_by_id(&tab_id);
        }
        self.activate_tab()?;
        self.save()?;
        Ok(())
    }

    /// Find a child tab by nickname or index string
    pub fn find_child_tab(&self, user: &str) -> Option<&ChildTab> {
        // Try as index first
        if let Ok(idx) = user.parse::<usize>() {
            return self.child_tabs.get(idx);
        }
        // Then by nickname
        self.child_tabs.iter().find(|t| t.nickname == user)
    }

    /// Close a specific child tab by nickname or index
    pub fn close_child_tab(&mut self, user: &str) -> Result<()> {
        let idx = if let Ok(i) = user.parse::<usize>() {
            if i >= self.child_tabs.len() {
                bail!("子 tab 索引超出范围: {}（共 {} 个）", i, self.child_tabs.len());
            }
            i
        } else {
            self.child_tabs
                .iter()
                .position(|t| t.nickname == user)
                .ok_or_else(|| anyhow::anyhow!("未找到用户: {}", user))?
        };
        let child = self.child_tabs.remove(idx);
        let _ = axcli::close_tab_by_id(&child.tab_id);
        self.save()?;
        Ok(())
    }

}

fn sessions_dir() -> Result<PathBuf> {
    let dir = dirs::home_dir()
        .context("cannot find home directory")?
        .join(".ax-xhs-cli")
        .join("sessions");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn session_path(id: &str) -> Result<PathBuf> {
    Ok(sessions_dir()?.join(format!("{}.json", id)))
}

impl Session {
    pub fn save(&self) -> Result<()> {
        let path = session_path(&self.id)?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(id: &str) -> Result<Self> {
        let path = session_path(id)?;
        let json = fs::read_to_string(&path)
            .with_context(|| format!("session '{}' not found", id))?;
        let session: Self = serde_json::from_str(&json)?;
        Ok(session)
    }

    pub fn delete(&self) -> Result<()> {
        let path = session_path(&self.id)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

/// List all sessions
pub fn list_sessions() -> Result<Vec<Session>> {
    let dir = sessions_dir()?;
    let mut sessions = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            if let Ok(json) = fs::read_to_string(&path) {
                if let Ok(session) = serde_json::from_str::<Session>(&json) {
                    sessions.push(session);
                }
            }
        }
    }
    sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(sessions)
}

/// Get the active session (auto-select if only one exists) and activate its tab
pub fn get_active_session(session_id: Option<&str>) -> Result<Session> {
    let session = if let Some(id) = session_id {
        Session::load(id)?
    } else {
        bail!("请通过 --session <NAME> 指定 session\n\n查看所有 session: ax-xhs-cli session list\n创建新 session:   ax-xhs-cli session start <NAME>")
    };

    // Activate the session's tab in Chrome
    session.activate_tab()?;

    Ok(session)
}

/// Create a new session with a given name
pub fn start_session(name: &str) -> Result<Session> {
    // Check if session with this name already exists
    if session_path(name)?.exists() {
        bail!("session '{}' 已存在，请使用其他名称或先 end 该 session", name);
    }

    let mut session = Session::new(name);

    // Collect tab IDs already used by existing sessions
    let used_tab_ids: Vec<String> = list_sessions()?
        .iter()
        .filter_map(|s| s.tab_id.clone())
        .collect();

    // Always open a new tab for this session to avoid sharing tabs
    axcli::open_url("https://www.xiaohongshu.com/explore")?;

    // Bind to the current (newly opened) tab
    session.bind_to_current_tab()?;

    // Safety check: if the tab ID is already used by another session, something went wrong
    if let Some(ref tab_id) = session.tab_id {
        if used_tab_ids.contains(tab_id) {
            // The open_url might have reused a tab; try opening again
            axcli::open_url("https://www.xiaohongshu.com/explore")?;
            session.bind_to_current_tab()?;
        }
    }

    session.detect_page_type()?;
    session.save()?;

    Ok(session)
}
