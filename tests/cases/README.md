# 冒烟测试执行指南

## 环境准备

```bash
# 构建
cargo build

# CLI 路径
CLI=./target/debug/ax-xhs-cli

# 清理旧 session
rm -f ~/.ax-xhs-cli/sessions/*.json

# 确认 Chrome 已打开且已登录小红书
$CLI status
```

## 测试用例

| # | 文件 | 覆盖内容 |
|---|------|----------|
| 01 | session_lifecycle | Session 创建/列表/状态/关闭、多 session 并行 |
| 02 | login | 登录状态检测 |
| 03 | search | 关键词搜索、排序、筛选 |
| 04 | note_and_comments | 笔记详情（text/json）、评论查看 |
| 05 | user_profile | 用户主页、子 tab 管理（打开/列表/操作/关闭） |
| 06 | feeds | 首页推荐、场景切换 |
| 07 | like_favorite | 点赞/取消点赞、收藏/取消收藏、状态检测 |
| 08 | pagination | 分页滚动、offset、增量采集、大 size 采集 |
| 09 | edge_cases | 错误处理（越界、缺参数、重复创建、tab 关闭） |
| 10 | notification | 通知 tab（评论和@、赞和收藏）、从通知打开用户 |
| 11 | scene_params | 场景参数一致性校验、省略/相同/不同参数 |

## 执行规则

1. **按编号顺序执行** — 后续用例可能依赖前面用例创建的 session 和数据
2. **所有索引从 0 开始** — `show-note 0` 是第一条
3. **命令结构** — 场景命令 + 可选子命令：
   ```bash
   $CLI --session S search -k "关键词" --size 10      # 进入搜索场景
   $CLI --session S search show-note 0                  # 在搜索场景中操作
   $CLI --session S feeds --size 5                      # 进入首页推荐场景
   $CLI --session S notification --scene-tab "赞和收藏" # 进入通知场景
   ```
4. **场景参数以 `--scene-` 为前缀** — 如 `--scene-sort`、`--scene-tab`、`--scene-name`
5. **每个测试文件内的 Case 按顺序执行**，Case 之间可能有依赖
6. **标记为"跳过"的 Case**（如发评论）有副作用，需手动确认后执行

## 验证手段

- **CLI 输出**：检查 stdout/stderr 文本
- **Session 文件**：读取 `~/.ax-xhs-cli/sessions/<name>.json`
- **Tab 状态**：用 AppleScript 检查 Chrome tab

### 判断规则
- "输出包含 X"：stdout 中存在子串 X 即通过
- "输出为 JSON"：stdout 可被 `jq` 解析即通过
- "报错"：exit code 非 0 且 stderr 包含预期错误信息

## 快速冒烟测试

```bash
CLI=./target/debug/ax-xhs-cli
$CLI status
rm -f ~/.ax-xhs-cli/sessions/*.json
$CLI session start "test"
$CLI --session test search -k "编程" --scene-sort "最新" --size 20
$CLI --session test search show-note 0
$CLI --session test search show-note 15
$CLI --session test search show-user 0 --size 3
$CLI --session test user-profile list
$CLI --session test feeds --size 5
$CLI --session test feeds show-note 0
$CLI --session test notification --scene-tab "赞和收藏"
$CLI session end "test"
```
