use anyhow::Result;
use crate::axcli;

/// Check XHS status globally (no session required)
pub fn check_status() -> Result<()> {
    let has_xhs = axcli::exists("webarea[title*=\"小红书\"]");

    if !has_xhs {
        // 自动打开小红书，等待页面加载，检查后关闭
        axcli::open_url("https://www.xiaohongshu.com/explore")?;
        let tab_id = axcli::get_active_tab_id()?;

        // 轮询等待页面加载（最多 10 秒）
        let mut page_loaded = false;
        for _ in 0..20 {
            if axcli::exists("webarea[title*=\"小红书\"]") {
                page_loaded = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        let logged_in = if page_loaded {
            axcli::exists("link[desc=\"我\"]")
        } else {
            false
        };
        let _ = axcli::close_tab_by_id(&tab_id);

        if logged_in {
            println!("已登录");
        } else {
            println!("未登录");
            println!("  请在浏览器中手动登录小红书（扫码或账号密码）");
        }
        return Ok(());
    }

    let logged_in = axcli::exists("link[desc=\"我\"]");

    if logged_in {
        println!("已登录");
    } else {
        println!("未登录");
        println!("  请在浏览器中手动登录小红书（扫码或账号密码）");
    }

    Ok(())
}
