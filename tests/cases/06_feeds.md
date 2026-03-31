# 首页推荐测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/feeds-test.json
ax-xhs-cli session start "feeds-test"
```

---

## Case 1: 获取首页推荐

### 步骤
1. 执行: `ax-xhs-cli --session feeds-test feeds --size 5`

### 预期结果
- 输出至少 5 行 `[N] 标题 - 作者 ❤数字`
- 末尾显示 "共 N 条推荐 (session: feeds-test)"

---

## Case 2: 从推荐查看笔记

### 步骤
1. 执行: `ax-xhs-cli --session feeds-test feeds show-note 0`

### 预期结果
- 输出笔记详情（标题、作者、链接、内容）

---

## Case 3: 从搜索切换回首页

### 步骤
1. 执行: `ax-xhs-cli --session feeds-test search -k "编程" --size 3`
2. 执行: `ax-xhs-cli --session feeds-test feeds --size 5`

### 预期结果
- 步骤 2：正常返回首页推荐结果（自动点 logo 回首页）

## Cleanup
```bash
ax-xhs-cli session end "feeds-test"
```
