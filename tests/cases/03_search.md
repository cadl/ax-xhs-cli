# 搜索测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/search-test.json
ax-xhs-cli session start "search-test"
```

---

## Case 1: 关键词搜索

### 步骤
1. 执行: `ax-xhs-cli --session search-test search -k "编程" --size 5`

### 预期结果
- 输出至少 5 行 `[N] 标题 - 作者 ❤数字`（N 从 0 开始）
- 末尾显示 "共 N 条搜索结果 (session: search-test)"

---

## Case 2: 切换关键词重新搜索

### 步骤
1. 执行: `ax-xhs-cli --session search-test search -k "旅行" --size 5`

### 预期结果
- 结果列表与 Case 1 不同

---

## Case 3: 排序 + 筛选组合

可用排序值: `综合`、`最新`、`最多点赞`、`最多评论`、`最多收藏`

### 步骤
1. 执行: `ax-xhs-cli --session search-test search -k "美食" --scene-sort "最多点赞" --size 5`
2. 执行: `ax-xhs-cli --session search-test search -k "美食" --scene-sort "最新" --size 5`

### 预期结果
- 步骤 1：结果按点赞数排序（❤ 数字整体较高）
- 步骤 2：结果按时间排序，内容与步骤 1 不同

---

## Case 4: 筛选笔记类型

### 步骤
1. 执行: `ax-xhs-cli --session search-test search -k "编程" --scene-note-type "视频" --size 5`

### 预期结果
- 返回结果（视频类笔记）

## Cleanup
```bash
ax-xhs-cli session end "search-test"
```
