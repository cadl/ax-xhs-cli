mod axcli;
mod commands;
mod mouse;
mod output;
mod parser;
mod session;

use clap::{Parser, Subcommand};
use output::OutputFormat;

#[derive(Parser)]
#[command(name = "ax-xhs-cli")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "通过 axcli 自动化操作小红书（基于 macOS Accessibility API）")]
#[command(after_help = r#"概念说明:

  Session:
    所有操作需要在 session 内进行（通过 --session 指定）。
    session 绑定一个 Chrome tab，维护页面状态和结果缓存。
    创建: session start <name>  结束: session end <name>

  场景 (Scene):
    命令按页面场景组织: search、search-user、feeds、user-profile、notification、open-note、open-user。
    每个场景有自己的子命令（如 show-note、like-note 等）。
    不带子命令时进入/刷新场景；带子命令时在当前场景下操作。

  场景参数 (--scene-xxx):
    以 --scene- 为前缀的参数定义场景上下文（如 --scene-keyword、--scene-tab）。
    场景参数会保存到 session 中。子命令执行时：
    - 不传场景参数 → 使用 session 中保存的参数
    - 传入相同参数 → 正常执行
    - 传入不同参数 → 报错（需先不带子命令执行场景命令来切换场景）
    --size 不是场景参数，不影响场景状态。

  子 Tab:
    search/feeds show-user、notification show-user、open-user 会打开用户页子 tab。
    子 tab 保留至 session end，期间可通过 user-profile --scene-name 操作。
    user-profile list 查看所有子 tab，user-profile close 关闭。

  输出格式:
    全局 -f/--format 参数，支持 text（默认）、json、yaml。

典型用法:
  ax-xhs-cli session start demo
  ax-xhs-cli --session demo search --scene-keyword "编程" --scene-sort "最新"
  ax-xhs-cli --session demo search show-note 0
  ax-xhs-cli --session demo search show-user 0
  ax-xhs-cli --session demo search-user -k "关键词"
  ax-xhs-cli --session demo search-user show-user 0
  ax-xhs-cli --session demo user-profile --scene-name "用户名" show-note 0
  ax-xhs-cli --session demo feeds show-note 0
  ax-xhs-cli --session demo notification --scene-tab "赞和收藏"
  ax-xhs-cli --session demo open-note "<完整URL含xsec_token>" like-note
  ax-xhs-cli --session demo session end
"#)]
struct Cli {
    /// 指定 session ID
    #[arg(long, global = true)]
    session: Option<String>,

    /// 输出格式
    #[arg(long, short = 'f', global = true, default_value = "text")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Session 管理
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// 检查小红书登录状态（无需 session）
    Status,

    // --- 场景命令 ---
    /// 搜索笔记
    Search {
        /// [场景参数] 搜索关键词
        #[arg(long = "scene-keyword", short = 'k')]
        keyword: Option<String>,
        /// [场景参数] 排序依据
        #[arg(long = "scene-sort", value_parser = ["综合", "最新", "最多点赞", "最多评论", "最多收藏"])]
        sort: Option<String>,
        /// [场景参数] 笔记类型
        #[arg(long = "scene-note-type", value_parser = ["不限", "视频", "图文"])]
        note_type: Option<String>,
        /// [场景参数] 发布时间
        #[arg(long = "scene-time", value_parser = ["不限", "一天内", "一周内", "半年内"])]
        time: Option<String>,
        /// [场景参数] 搜索范围
        #[arg(long = "scene-scope", value_parser = ["不限", "已看过", "未看过", "已关注"])]
        scope: Option<String>,
        /// [场景参数] 位置距离
        #[arg(long = "scene-location", value_parser = ["不限", "同城", "附近"])]
        location: Option<String>,
        /// 返回结果数量
        #[arg(long, default_value = "20")]
        size: usize,
        /// 操作子命令
        #[command(subcommand)]
        action: Option<commands::actions::NoteAction>,
    },
    /// 获取首页推荐
    Feeds {
        /// 返回结果数量
        #[arg(long, default_value = "20")]
        size: usize,
        /// 操作子命令
        #[command(subcommand)]
        action: Option<commands::actions::NoteAction>,
    },
    /// 用户主页（操作已打开的子 tab）
    #[command(name = "user-profile")]
    UserProfile {
        /// [场景参数] 用户昵称或子 tab 索引
        #[arg(long = "scene-name")]
        name: Option<String>,
        /// 返回笔记数量
        #[arg(long, default_value = "20")]
        size: usize,
        /// 操作子命令
        #[command(subcommand)]
        action: Option<commands::user_profile::UserProfileAction>,
    },
    /// 查看通知
    Notification {
        /// [场景参数] 通知分类 tab
        #[arg(long = "scene-tab", value_parser = ["评论和@", "赞和收藏", "新增关注"])]
        tab: Option<String>,
        /// 操作子命令
        #[command(subcommand)]
        action: Option<commands::notification::NotificationAction>,
    },

    /// 搜索用户
    #[command(name = "search-user")]
    SearchUser {
        /// [场景参数] 搜索关键词
        #[arg(long = "scene-keyword", short = 'k')]
        keyword: Option<String>,
        /// 返回结果数量
        #[arg(long, default_value = "20")]
        size: usize,
        /// 操作子命令
        #[command(subcommand)]
        action: Option<commands::search_user::SearchUserAction>,
    },

    // --- URL 直接访问场景 ---
    /// 通过 URL 打开笔记（需要完整 URL，包含 xsec_token 参数）
    #[command(name = "open-note", after_help = "URL 必须包含 xsec_token 参数，否则页面无法加载。\n可从 search/feeds show-note 输出的 url 字段获取完整链接。")]
    OpenNote {
        /// 笔记完整 URL（含 xsec_token）
        url: String,
        /// 操作子命令
        #[command(subcommand)]
        action: Option<commands::open::OpenNoteAction>,
    },
    /// 通过 URL 打开用户主页
    #[command(name = "open-user")]
    OpenUser {
        /// 用户主页 URL
        url: String,
        /// 返回笔记数量
        #[arg(long, default_value = "20")]
        size: usize,
    },

    // --- 独立命令 ---
    /// 检查 AX 树结构（调试用）
    Inspect {
        /// axcli locator（不传则检查整个 Chrome）
        locator: Option<String>,
        /// 树深度
        #[arg(long, default_value = "5")]
        depth: usize,
    },
    /// 测试人类鼠标轨迹点击（调试用）
    #[command(hide = true)]
    TestClick {
        /// axcli 选择器
        locator: String,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// 创建新 session（打开/关联 XHS tab）
    Start {
        /// Session 名称（用作 ID 和描述）
        name: String,
    },
    /// 列出所有 session
    List,
    /// 结束 session（关闭 tab）
    End {
        /// Session 名称（可选，单 session 时自动选择）
        name: Option<String>,
    },
    /// 查看 session 状态
    Status,
}

fn main() {
    let cli = Cli::parse();
    let session_id = cli.session.as_deref();
    let format = cli.format;

    let result = match cli.command {
        Commands::Session { action } => match action {
            SessionAction::Start { name } => commands::session_cmd::start(&name, format),
            SessionAction::List => commands::session_cmd::list(format),
            SessionAction::End { name } => {
                commands::session_cmd::end(name.as_deref().or(session_id), format)
            }
            SessionAction::Status => commands::session_cmd::status(session_id, format),
        },
        Commands::Status => commands::login::check_status(),
        Commands::Search {
            keyword,
            sort,
            note_type,
            time,
            scope,
            location,
            size,
            action,
        } => commands::search::search(
            session_id,
            keyword.as_deref(),
            sort.as_deref(),
            note_type.as_deref(),
            time.as_deref(),
            scope.as_deref(),
            location.as_deref(),
            size,
            action,
            format,
        ),
        Commands::SearchUser {
            keyword,
            size,
            action,
        } => commands::search_user::search_user(
            session_id,
            keyword.as_deref(),
            size,
            action,
            format,
        ),
        Commands::Feeds {
            size,
            action,
        } => commands::feeds::feeds(session_id, size, action, format),
        Commands::UserProfile {
            name,
            size,
            action,
        } => commands::user_profile::user_profile(
            session_id,
            name.as_deref(),
            size,
            action,
            format,
        ),
        Commands::Notification { tab, action } => {
            commands::notification::notification(session_id, tab.as_deref(), action, format)
        }
        Commands::OpenNote { url, action } => {
            commands::open::open_note(session_id, &url, action, format)
        }
        Commands::OpenUser { url, size } => {
            commands::open::open_user(session_id, &url, size, format)
        }
        Commands::Inspect { locator, depth } => {
            commands::inspect::inspect(locator.as_deref(), depth, format)
        }
        Commands::TestClick { locator } => {
            axcli::human_click(&locator).map(|msg| println!("{}", msg))
        }
    };

    if let Err(e) = result {
        eprintln!("错误: {:#}", e);
        std::process::exit(1);
    }
}
