# 点赞与收藏测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/like-test.json
ax-xhs-cli session start "like-test"
ax-xhs-cli --session like-test search -k "编程" --size 5
```

---

## Case 1: 检查当前点赞/收藏状态

### 步骤
1. 执行: `ax-xhs-cli --session like-test search show-note 0 -f json`

### 预期结果
- JSON 中 `liked` 和 `favorited` 为 boolean 值

---

## Case 2: 点赞（未点赞时）

### 步骤
1. 执行: `ax-xhs-cli --session like-test search like-note 0`

### 预期结果
- 若未点赞：输出 "点赞成功"
- 若已点赞：输出 "已经点赞过了"

---

## Case 3: 重复点赞

### 步骤
1. 执行: `ax-xhs-cli --session like-test search like-note 0`

### 预期结果
- 输出 "已经点赞过了"

---

## Case 4: 取消点赞

### 步骤
1. 执行: `ax-xhs-cli --session like-test search unlike-note 0`

### 预期结果
- 若已点赞：输出 "已取消点赞"
- 若未点赞：输出 "当前未点赞，无需取消"

---

## Case 5: 收藏与取消收藏

### 步骤
1. 执行: `ax-xhs-cli --session like-test search favorite-note 0`
2. 执行: `ax-xhs-cli --session like-test search unfavorite-note 0`

### 预期结果
- 步骤 1：输出 "收藏成功" 或 "已经收藏过了"
- 步骤 2：输出 "已取消收藏" 或 "当前未收藏，无需取消"

## Cleanup
```bash
ax-xhs-cli session end "like-test"
```
