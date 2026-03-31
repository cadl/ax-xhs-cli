# 用户主页与子 Tab 测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/user-test.json
ax-xhs-cli session start "user-test"
ax-xhs-cli --session user-test search -k "编程" --size 5
```

---

## Case 1: 从搜索结果打开用户主页

### 步骤
1. 执行: `ax-xhs-cli --session user-test search show-user 0 --size 3`

### 预期结果
- 输出用户信息：昵称、小红书号、关注/粉丝数
- 输出该用户的笔记列表（最多 3 条）

---

## Case 2: 列出已打开的子 Tab

### 步骤
1. 执行: `ax-xhs-cli --session user-test user-profile list`

### 预期结果
- 输出包含 Case 1 打开的用户昵称

---

## Case 3: 在用户主页操作笔记

### 步骤
1. 执行: `ax-xhs-cli --session user-test user-profile --scene-name "<Case 1 的用户昵称>" show-note 0`

### 预期结果
- 输出该用户的笔记详情（标题、作者、链接、内容）

---

## Case 4: 打开第二个用户

### 步骤
1. 执行: `ax-xhs-cli --session user-test search show-user 1 --size 3`
2. 执行: `ax-xhs-cli --session user-test user-profile list`

### 预期结果
- 步骤 2：list 显示两个用户

---

## Case 5: 关闭用户子 Tab

### 步骤
1. 执行: `ax-xhs-cli --session user-test user-profile close "<用户昵称>"`
2. 执行: `ax-xhs-cli --session user-test user-profile list`

### 预期结果
- 步骤 1：输出 "已关闭"
- 步骤 2：list 中不再包含该用户

## Cleanup
```bash
ax-xhs-cli session end "user-test"
```
