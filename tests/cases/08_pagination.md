# 分页滚动测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/page-test.json
ax-xhs-cli session start "page-test"
```

---

## Case 1: 搜索指定数量

### 步骤
1. 执行: `ax-xhs-cli --session page-test search -k "编程" --size 5`
2. 执行: `ax-xhs-cli --session page-test search -k "编程" --size 20`

### 预期结果
- 步骤 1：返回 5 条结果
- 步骤 2：返回 20 条结果，前 5 条与步骤 1 一致（同场景不重搜，增量滚动）

---

## Case 2: 同关键词增量翻页

### 步骤
1. 执行: `ax-xhs-cli --session page-test search -k "编程" --size 10`

### 预期结果
- 返回 10 条结果，前 5 条与 Case 1 步骤 1 一致，第 6-10 条为增量滚动获取

---

## Case 3: feeds 分页

### 步骤
1. 执行: `ax-xhs-cli --session page-test feeds --size 10`
2. 执行: `ax-xhs-cli --session page-test feeds --size 20`

### 预期结果
- 步骤 2 返回 20 条，前 10 条与步骤 1 一致

---

## Case 4: 大 size 滚动采集

### 步骤
1. 执行: `ax-xhs-cli --session page-test search -k "旅行" --size 30`

### 预期结果
- 返回 30 条结果（需多次滚动采集）
- 索引 0-29 连续

## Cleanup
```bash
ax-xhs-cli session end "page-test"
```
