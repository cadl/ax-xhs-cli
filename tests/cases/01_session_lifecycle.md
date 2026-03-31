# Session 生命周期测试

## Setup
```bash
rm -f ~/.ax-xhs-cli/sessions/*.json
```

---

## Case 1: 创建、查看、关闭 session 完整流程

### 步骤
1. 执行: `ax-xhs-cli session start "生命周期测试"`
2. 执行: `ax-xhs-cli session list`
3. 执行: `ax-xhs-cli --session "生命周期测试" session status`
4. 执行: `ax-xhs-cli session end "生命周期测试"`
5. 执行: `ax-xhs-cli session list`

### 预期结果
- 步骤 1：输出包含 "Session '生命周期测试' started"，包含 "Tab ID:"
- 步骤 2：输出包含 "生命周期测试"
- 步骤 3：输出包含 "Session:  生命周期测试"，包含 "Page:" 和 "Tab ID:"
- 步骤 4：输出包含 "Session '生命周期测试' ended"
- 步骤 5：输出包含 "没有活跃的 session"

---

## Case 2: 重复创建同名 session

### 步骤
1. 执行: `ax-xhs-cli session start "重复测试"`
2. 执行: `ax-xhs-cli session start "重复测试"`

### 预期结果
- 步骤 2：报错，stderr 包含 "已存在"

### Cleanup
```bash
ax-xhs-cli session end "重复测试"
```

---

## Case 3: 无 session 时执行业务命令

### 步骤
1. 执行: `ax-xhs-cli search -k "测试"`

### 预期结果
- 报错，stderr 包含 "请通过 --session <NAME> 指定 session"

---

## Case 4: 多 Session 并行

### 步骤
1. 执行: `ax-xhs-cli session start "A"`
2. 执行: `ax-xhs-cli session start "B"`
3. 执行: `ax-xhs-cli --session A search -k "编程" --size 3`
4. 执行: `ax-xhs-cli --session B search -k "旅行" --size 3`
5. 执行: `ax-xhs-cli session list`
6. 执行: `ax-xhs-cli session end "A"`
7. 执行: `ax-xhs-cli --session B session status`

### 预期结果
- 步骤 3-4：各自独立搜索
- 步骤 5：list 显示 A 和 B
- 步骤 6：仅结束 A
- 步骤 7：B 仍正常，status 显示其搜索状态

### Cleanup
```bash
ax-xhs-cli session end "B"
```
