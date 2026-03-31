# 场景参数一致性测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/param-test.json
ax-xhs-cli session start "param-test"
ax-xhs-cli --session param-test search -k "编程" --scene-sort "最新" --size 5
```

---

## Case 1: 子命令省略场景参数（使用已保存的）

### 步骤
1. 执行: `ax-xhs-cli --session param-test search show-note 0`（不带 -k）

### 预期结果
- 正常执行，使用 session 中保存的 keyword 和 sort

---

## Case 2: 子命令带相同参数（不重搜）

### 步骤
1. 执行: `ax-xhs-cli --session param-test search -k "编程" show-note 0`

### 预期结果
- 不重新搜索，直接在已有结果上操作

---

## Case 3: 子命令带不同关键词（报错）

### 步骤
1. 执行: `ax-xhs-cli --session param-test search -k "旅行" show-note 0`

### 预期结果
- 报错，提示 "场景参数不一致"

---

## Case 4: 不带子命令切换场景（正常）

### 步骤
1. 执行: `ax-xhs-cli --session param-test search -k "旅行" --size 5`

### 预期结果
- 正常切换到新关键词搜索

---

## Case 5: open-note 场景参数

### 步骤
1. 获取笔记 URL：`ax-xhs-cli --session param-test search show-note 0 -f json`（取 url 字段）
2. 执行: `ax-xhs-cli --session param-test open-note "<URL>" show-comments --size 3`

### 预期结果
- 打开指定 URL 笔记并显示评论

## Cleanup
```bash
ax-xhs-cli session end "param-test"
```
