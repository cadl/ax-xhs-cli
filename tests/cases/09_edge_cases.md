# 错误处理与边界测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/edge-test.json
ax-xhs-cli session start "edge-test"
ax-xhs-cli --session edge-test search -k "编程" --size 3
```

---

## Case 1: 无 session 执行命令

### 步骤
1. 执行: `ax-xhs-cli search -k "test"`

### 预期结果
- 报错，提示 "请通过 --session <NAME> 指定 session"

---

## Case 2: 索引越界

### 步骤
1. 执行: `ax-xhs-cli --session edge-test search show-note 999`

### 预期结果
- 报错，提示 "索引超出范围"

---

## Case 3: 无已有场景时缺少必须参数

### 步骤
1. 执行: `ax-xhs-cli session start "fresh-test"`
2. 执行: `ax-xhs-cli --session fresh-test search`（不带 -k，且无已保存的关键词）

### 预期结果
- 步骤 2：报错，提示缺少 keyword
- 注意：若 session 已有保存的关键词，省略 -k 会复用已保存的参数，属正常行为

### Cleanup
```bash
ax-xhs-cli session end "fresh-test"
```

---

## Case 4: 不存在的用户子 Tab

### 步骤
1. 执行: `ax-xhs-cli --session edge-test user-profile --scene-name "不存在的用户" show-note 0`

### 预期结果
- 报错，提示 "未找到用户"

---

## Case 5: 重复创建同名 session

### 步骤
1. 执行: `ax-xhs-cli session start "edge-test"`

### 预期结果
- 报错，提示 "已存在"

---

## Case 6: Session 绑定 tab 被关闭

### 步骤
1. 执行: `ax-xhs-cli session start "tab-test"`
2. 手动关闭该 tab：`osascript -e 'tell application "Google Chrome" to close active tab of front window'`
3. 执行: `ax-xhs-cli --session "tab-test" search -k "test"`

### 预期结果
- 步骤 3 报错，包含 "可能已关闭"

### Cleanup
```bash
rm -f ~/.ax-xhs-cli/sessions/tab-test.json
```

## Cleanup
```bash
ax-xhs-cli session end "edge-test"
```
