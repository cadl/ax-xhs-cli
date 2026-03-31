# 通知场景测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/notif-test.json
ax-xhs-cli session start "notif-test"
```

---

## Case 1: 查看通知（评论和@）

### 步骤
1. 执行: `ax-xhs-cli --session notif-test notification --scene-tab "评论和@"`

### 预期结果
- 输出通知列表，每条包含用户名和内容摘要

---

## Case 2: 切换通知 Tab

### 步骤
1. 执行: `ax-xhs-cli --session notif-test notification --scene-tab "赞和收藏"`

### 预期结果
- 输出切换后的通知列表

---

## Case 3: 从通知打开用户主页

### 步骤
1. 执行: `ax-xhs-cli --session notif-test notification --scene-tab "评论和@" show-user 0`

### 预期结果
- 输出用户信息和笔记列表
- `user-profile list` 中可见该用户

---

## Case 4: 省略 scene-tab（复用已保存的）

### 步骤
1. 执行: `ax-xhs-cli --session notif-test notification show-user 0`

### 预期结果
- 使用上次保存的 scene-tab，正常执行

## Cleanup
```bash
ax-xhs-cli session end "notif-test"
```
