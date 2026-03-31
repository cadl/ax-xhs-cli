# 登录检查测试

## 前置条件
- Chrome 已打开，用户已登录小红书

---

## Case 1: 已有小红书 tab 时检查登录状态

### 步骤
1. 执行: `ax-xhs-cli session start "登录测试"`
2. 执行: `ax-xhs-cli status`

### 预期结果
- 步骤 2：输出包含 "已登录"

### 验证方法
- 用 `axcli snapshot --app "Google Chrome" 'link[desc="我"]'` 验证侧边栏有"我"链接（表示已登录）

### 清理
- `ax-xhs-cli session end "登录测试"`

---

## Case 2: 无小红书 tab 时自动打开检查

### 前置条件
- Chrome 中没有打开小红书的 tab

### 步骤
1. 关闭所有小红书 tab（如有）
2. 执行: `ax-xhs-cli status`

### 预期结果
- 输出包含 "已登录"
- 命令执行期间会自动打开小红书页面，检查完毕后自动关闭
- 不会遗留小红书 tab

### 验证方法
- 用 AppleScript 检查 Chrome 中不存在 title 包含 "小红书" 的 tab（自动打开的已被关闭）
