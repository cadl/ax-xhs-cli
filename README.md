# ax-xhs-cli

通过 macOS Accessibility API 自动化操作小红书（Xiaohongshu / RED）的命令行工具，使用 Rust 编写。

不逆向小红书，不注入脚本，使用 Accessibility API 模拟操作现有的 Chrome。项目使用自然语言功能测试集，使用类 SDD 方式，检验功能是否正常，并跟随页面结构变化。

## 快速验证

构建完成后，确保 Chrome 已打开并登录小红书，然后使用 [Claude Code](https://claude.ai/claude-code)、[Codex](https://openai.com/codex) 等 AI 编程工具执行测试：

```
请读取 tests/cases/README.md 了解执行规则，然后依次执行 tests/cases/ 目录下的冒烟测试
```

测试用例为自然语言描述（非代码），覆盖 11 个场景：session 生命周期、搜索与筛选、笔记详情与评论、用户主页与子 tab、首页推荐、点赞/收藏、分页滚动、错误处理、通知、场景参数一致性等。

https://github.com/user-attachments/assets/8d179db0-264b-4910-a05c-ed31aaa96e96

> 2x 速度播放的冒烟测试录屏

## 核心概念

### Session

所有操作在 session 内进行。session 绑定一个 Chrome tab，维护页面状态、搜索结果缓存和子 tab。

```bash
ax-xhs-cli session start "demo"     # 创建 session
ax-xhs-cli --session demo ...       # 在 session 内操作
ax-xhs-cli session end "demo"       # 结束 session
```

### 场景 (Scene)

命令按页面场景组织，每个场景有自己的子命令：

不带子命令时进入/刷新场景；带子命令时在当前场景下操作。

| 场景 | 说明 | 场景参数 | show-note | show-user | like | unlike | favorite | unfavorite | comment | show-comments | list | close |
|------|------|----------|:---------:|:---------:|:----:|:------:|:--------:|:----------:|:-------:|:-------------:|:----:|:-----:|
| `search` | 搜索笔记 | `--scene-keyword` (`-k`), `--scene-sort`, `--scene-note-type`, `--scene-time`, `--scene-scope`, `--scene-location` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| `feeds` | 首页推荐 | — | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| `user-profile` | 用户主页 | `--scene-name` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `notification` | 通知 | `--scene-tab` (评论和@, 赞和收藏, 新增关注) | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `open-note` | URL 打开笔记 | `<URL>` (位置参数，需含 xsec_token) | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ❌ | ❌ |
| `open-user` | URL 打开用户 | `<URL>` (位置参数) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

### 子 Tab

`show-user`（search/feeds/notification）和 `open-user` 会打开用户页子 tab，保留至 session end：

```bash
ax-xhs-cli --session demo search show-user 0           # 打开用户页
ax-xhs-cli --session demo user-profile list              # 查看子 tab
ax-xhs-cli --session demo user-profile --scene-name "用户名" show-note 0  # 操作
ax-xhs-cli --session demo user-profile close "用户名"    # 关闭
```

### 输出格式

全局 `-f`/`--format` 参数：`text`（默认）、`json`、`yaml`。

### 点赞/收藏状态检测

通过截图颜色检测 icon 状态（红心=已点赞，黄星=已收藏），操作前先判断，避免误触。

## 前置要求

- macOS（依赖 Accessibility API 和 CoreGraphics）
- Google Chrome
- 以下应用需在 系统设置 → 隐私与安全性 → 辅助功能 中授权：
  - **终端 app**（Terminal / iTerm2 等）— 操作 UI 元素、模拟键鼠
  - **Google Chrome** — 暴露完整的无障碍树，否则无法定位页面元素
- 运行本 CLI 的终端 app 还需授权：
  - **屏幕录制** — 截图检测点赞/收藏状态（颜色识别）

## 安装

```bash
cargo install ax-xhs-cli
```

或从源码构建：

```bash
cargo build --release
cp target/release/ax-xhs-cli /usr/local/bin/
```

## 使用示例

```bash
# Session 管理
ax-xhs-cli session start "demo"

# 搜索场景
ax-xhs-cli --session demo search -k "编程" --scene-sort "最新" --size 10
ax-xhs-cli --session demo search show-note 0
ax-xhs-cli --session demo search show-note 0 -f json
ax-xhs-cli --session demo search show-user 0 --size 5
ax-xhs-cli --session demo search like-note 0
ax-xhs-cli --session demo search show-comments 0 --size 5

# 首页推荐场景
ax-xhs-cli --session demo feeds --size 10
ax-xhs-cli --session demo feeds show-note 0

# 用户主页场景
ax-xhs-cli --session demo user-profile list
ax-xhs-cli --session demo user-profile --scene-name "用户名" show-note 0
ax-xhs-cli --session demo user-profile close "用户名"

# 通知场景
ax-xhs-cli --session demo notification --scene-tab "赞和收藏"
ax-xhs-cli --session demo notification --scene-tab "评论和@" show-user 0

# URL 直接访问
ax-xhs-cli --session demo open-note "<完整URL含xsec_token>"
ax-xhs-cli --session demo open-note "<URL>" like-note
ax-xhs-cli --session demo open-note "<URL>" show-comments --size 10
ax-xhs-cli --session demo open-user "https://www.xiaohongshu.com/user/profile/xxx"

# AX 树检查
ax-xhs-cli inspect ".feeds-container" --depth 3

# 结束
ax-xhs-cli session end "demo"
```

## 命令参考

### 全局参数

| 参数 | 说明 |
|------|------|
| `--session <NAME>` | 指定 session ID |
| `-f, --format <FORMAT>` | 输出格式：text / json / yaml |

### `session` — Session 管理

| 子命令 | 说明 |
|--------|------|
| `session start <NAME>` | 创建 session |
| `session list` | 列出所有 session |
| `session end [NAME]` | 结束 session |
| `session status` | 查看 session 状态 |

### `search` — 搜索笔记

```
search [--scene-keyword <K>] [--scene-sort <S>] [--scene-note-type <T>]
       [--scene-time <T>] [--scene-scope <S>] [--scene-location <L>]
       [--size N] [SUBCOMMAND]
```

场景参数：`--scene-keyword`（`-k`）, `--scene-sort`, `--scene-note-type`, `--scene-time`, `--scene-scope`, `--scene-location`

### `feeds` — 首页推荐

```
feeds [--size N] [SUBCOMMAND]
```

无场景参数。已在首页时直接滚动加载。

### `user-profile` — 用户主页

```
user-profile [--scene-name <NAME>] [--size N] [SUBCOMMAND]
user-profile list
user-profile close <NAME>
```

操作已打开的子 tab。`--scene-name` 指定用户昵称或子 tab 索引。

### `notification` — 通知

```
notification [--scene-tab <TAB>] [SUBCOMMAND]
```

场景参数：`--scene-tab`（评论和@、赞和收藏、新增关注）。`show-user` 需要指定 `--scene-tab`。

### `open-note` — 通过 URL 打开笔记

```
open-note <URL> [SUBCOMMAND]
```

URL 必须包含 `xsec_token` 参数。可从 `show-note` 输出的 url 字段获取。
子命令：`like-note`, `unlike-note`, `favorite-note`, `unfavorite-note`, `show-comments`

### `open-user` — 通过 URL 打开用户主页

```
open-user <URL> [--size N]
```

先回首页，再以子 tab 打开用户页。打开后可通过 `user-profile` 操作。

### 通用子命令

search、feeds、user-profile 共享以下子命令：

| 子命令 | 说明 |
|--------|------|
| `show-note <INDEX>` | 查看笔记详情 |
| `show-user <INDEX>` | 打开用户主页（子 tab） |
| `like-note <INDEX>` | 点赞 |
| `unlike-note <INDEX>` | 取消点赞 |
| `favorite-note <INDEX>` | 收藏 |
| `unfavorite-note <INDEX>` | 取消收藏 |
| `comment-note <INDEX> -c <CONTENT>` | 评论 |
| `show-comments <INDEX> [--size N]` | 查看一级评论（总数含回复） |

### `inspect` — AX 树检查

```
inspect [LOCATOR] [--depth N]
```

### `status` — 登录检测

无需 session。

## 架构

```
src/
├── main.rs            # CLI 入口，clap 命令定义（场景 + 子命令）
├── axcli.rs           # axcli 库封装 + human_click + scroll + 截图颜色检测
├── mouse.rs           # CoreGraphics 鼠标轨迹 + 滚动
├── output.rs          # 输出格式化（text/json/yaml）
├── session.rs         # Session + ChildTab + 场景参数 + 状态机
├── parser.rs          # AXNode 树提取（笔记卡片、用户资料、评论、通知）
└── commands/
    ├── actions.rs     # 共享 NoteAction 子命令（show-note/like-note/...）
    ├── search.rs      # 搜索场景 + 分页滚动
    ├── feeds.rs       # 首页推荐场景
    ├── user_profile.rs # 用户主页场景（子 tab 管理）
    ├── notification.rs # 通知场景
    ├── open.rs        # URL 直接访问（open-note/open-user）
    ├── inspect.rs     # AX 树检查
    ├── login.rs       # 登录检测
    └── session_cmd.rs # session start/list/end/status
```

## 已知限制

- 仅支持 macOS（依赖 Accessibility API 和 CoreGraphics）
- 仅支持 Google Chrome（tab 管理通过 AppleScript 实现）
- Chrome 必须在运行，且终端 app 需有辅助功能权限
- 笔记 URL 需包含 `xsec_token` 参数才能直接打开
- 小红书页面结构可能随版本更新变化，选择器需要维护

## 致谢

- [axcli](https://github.com/andelf/axcli) — macOS Accessibility API 库（MIT / Apache-2.0）

## License

[MIT](LICENSE)
