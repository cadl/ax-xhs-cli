# 搜索用户测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/search-user-test.json
ax-xhs-cli session start "search-user-test"
```

---

## Case 1: 基本用户搜索

### 步骤
1. 执行: `ax-xhs-cli --session search-user-test search-user -k "编程" --size 5`

### 预期结果
- 输出用户列表，格式: `[N] 昵称 (小红书号: xxx) - 粉丝:N 笔记:N`
- 有简介的用户第二行缩进显示简介
- 末尾显示 "共 N 位用户 (session: search-user-test)"

---

## Case 2: 切换关键词重新搜索

### 步骤
1. 执行: `ax-xhs-cli --session search-user-test search-user -k "旅行" --size 5`

### 预期结果
- 结果列表与 Case 1 不同
- 格式一致

---

## Case 3: show-user 打开用户主页

### 步骤
1. 执行: `ax-xhs-cli --session search-user-test search-user -k "编程" --size 5`
2. 执行: `ax-xhs-cli --session search-user-test search-user show-user 0`

### 预期结果
- 输出用户主页信息（昵称、小红书号、简介、关注/粉丝/获赞与收藏）
- 输出用户笔记列表
- 用户注册为 child tab

### 验证
- 执行: `ax-xhs-cli --session search-user-test user-profile list`
- 应显示刚打开的用户

---

## Case 4: 场景参数一致性检查

### 步骤
1. 先执行搜索: `ax-xhs-cli --session search-user-test search-user -k "编程"`
2. 再用不同关键词带子命令: `ax-xhs-cli --session search-user-test search-user -k "旅行" show-user 0`

### 预期结果
- 第 2 步应报错: "场景参数不一致"

---

## Case 5: JSON 输出格式

### 步骤
1. 执行: `ax-xhs-cli --session search-user-test search-user -k "编程" --size 3 -f json`

### 预期结果
- 输出合法 JSON 数组
- 每个元素包含 index, name, xhs_id, description, followers, notes_count

---

## Case 6: 省略关键词复用 session 参数

### 步骤
1. 先执行搜索: `ax-xhs-cli --session search-user-test search-user -k "编程" --size 5`
2. 再执行不带关键词: `ax-xhs-cli --session search-user-test search-user show-user 0`

### 预期结果
- 第 2 步正常执行（复用 session 中保存的 keyword="编程"）

---

## Teardown
```bash
ax-xhs-cli session end "search-user-test"
```
