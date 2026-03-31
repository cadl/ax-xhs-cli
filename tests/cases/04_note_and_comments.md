# 笔记详情与评论测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/note-test.json
ax-xhs-cli session start "note-test"
ax-xhs-cli --session note-test search -k "编程" --size 5
```

---

## Case 1: 查看笔记详情（text 格式）

### 步骤
1. 执行: `ax-xhs-cli --session note-test search show-note 0`

### 预期结果
- 输出包含：标题、作者、链接（https://www.xiaohongshu.com/...）、正文内容
- 输出包含 ❤ 和 ⭐ 统计

---

## Case 2: 查看笔记详情（JSON 格式）

### 步骤
1. 执行: `ax-xhs-cli --session note-test search show-note 0 -f json`

### 预期结果
- 输出为合法 JSON
- 包含字段：`title`, `author`, `url`, `content`, `likes`, `favorites`, `liked`(bool), `favorited`(bool)

---

## Case 3: 查看不同索引的笔记

### 步骤
1. 执行: `ax-xhs-cli --session note-test search show-note 1`

### 预期结果
- 输出内容与 Case 1 不同（不同笔记）

---

## Case 4: 查看笔记评论

### 步骤
1. 执行: `ax-xhs-cli --session note-test search show-comments 0 --size 5`

### 预期结果
- 输出一级评论列表，每条包含作者和内容
- 末尾显示 "评论总数: 共 N 条评论（含回复）"（总数包含子级回复，列表只展示一级评论）

---

## Case 5: 评论 JSON 格式

### 步骤
1. 执行: `ax-xhs-cli --session note-test search show-comments 0 --size 3 -f json`

### 预期结果
- 合法 JSON，包含 `comments` 数组和 `total` 字段
- 每条评论有 `author` 和 `content`

---

## Case 6: 发表评论（跳过 - 不可逆操作）

> 需要手动确认后执行：`ax-xhs-cli --session note-test search comment-note 0 -c "[AX-TEST] 自动化测试评论"`

## Cleanup
```bash
ax-xhs-cli session end "note-test"
```
