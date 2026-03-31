use anyhow::{Context, Result, anyhow};
use std::process::Command;

use crate::mouse;

pub use axcli_lib::accessibility::AXNode;

const APP: &str = "Google Chrome";

// --- Chrome process + AXNode ---

fn chrome_pid() -> Result<i32> {
    let output = Command::new("pgrep")
        .args(["-x", APP])
        .output()
        .context("pgrep failed")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .ok_or_else(|| anyhow!("{} 未运行", APP))?
        .trim()
        .parse::<i32>()
        .context("invalid PID")
}

/// Get Chrome's root AXNode
pub fn chrome_root() -> Result<AXNode> {
    let pid = chrome_pid()?;
    Ok(AXNode::app(pid))
}

// --- Element access ---

/// Resolve a locator to an AXNode within Chrome
pub fn locate(locator: &str) -> Result<AXNode> {
    let root = chrome_root()?;
    root.locate(locator)
        .ok_or_else(|| anyhow!("element not found: {}", locator))
}

/// Resolve a locator, returning None if not found
pub fn locate_opt(locator: &str) -> Option<AXNode> {
    chrome_root().ok()?.locate(locator)
}

/// Check if an element matching the locator exists
pub fn exists(locator: &str) -> bool {
    locate_opt(locator).is_some()
}

// --- Actions ---

/// Click at a specific point with human-like mouse trajectory.
pub fn human_click_point(point: core_graphics::geometry::CGPoint) -> Result<()> {
    mouse::move_to(point, None)?;
    let pause: u64 = rand::random_range(500..1500);
    std::thread::sleep(std::time::Duration::from_millis(pause));
    mouse::click_at_current()?;
    Ok(())
}

/// Click an element with human-like mouse trajectory.
/// Moves mouse along a Bezier curve (with overshoot + correction) to the target,
/// pauses ~1s like a human confirming the target, then clicks via CGEvent.
pub fn human_click(locator: &str) -> Result<String> {
    let node = locate(locator)?;
    let center = element_center(&node, locator)?;

    // 1. Move mouse with human-like trajectory (Bezier + overshoot)
    mouse::move_to(center, None)?;

    // 2. Human pause before clicking: ~1s with 50% jitter (500-1500ms)
    let pause: u64 = rand::random_range(500..1500);
    std::thread::sleep(std::time::Duration::from_millis(pause));

    // 3. Click at current position via CGEvent
    mouse::click_at_current()?;

    Ok(format!("Human-clicked at ({:.0}, {:.0})", center.x, center.y))
}

fn element_center(
    node: &AXNode,
    locator: &str,
) -> Result<core_graphics::geometry::CGPoint> {
    let (px, py) = node
        .position()
        .ok_or_else(|| anyhow!("cannot get position: {}", locator))?;
    let (sx, sy) = node
        .size()
        .ok_or_else(|| anyhow!("cannot get size: {}", locator))?;
    Ok(core_graphics::geometry::CGPoint::new(
        px + sx / 2.0,
        py + sy / 2.0,
    ))
}

/// Hover over an element (move mouse to center, no click)
pub fn hover(locator: &str) -> Result<()> {
    let node = locate(locator)?;
    let center = element_center(&node, locator)?;
    mouse::move_to(center, None)?;
    Ok(())
}

/// Focus an element
pub fn focus(locator: &str) -> Result<()> {
    let node = locate(locator)?;
    node.set_focused(true);
    Ok(())
}

/// Type text into a focused element with human-like rhythm.
///
/// Chinese text appears in word-sized chunks (2-3 chars), simulating pinyin IME:
/// user types pinyin quickly → selects candidate → whole word appears at once.
/// ASCII text is typed character by character at normal touch-typing speed.
pub fn input(locator: &str, text: &str) -> Result<()> {
    focus(locator)?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    human_type(text);
    Ok(())
}

/// Type text with human-like rhythm, grouping Chinese into IME-style word chunks.
fn human_type(text: &str) {
    use std::time::Duration;

    let chunks = split_typing_chunks(text);
    for (i, chunk) in chunks.iter().enumerate() {
        let char_count = chunk.chars().count();

        // Typo chance: default 7%, override with AX_TYPO_RATE=0..100
        let typo_rate: u32 = std::env::var("AX_TYPO_RATE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7);
        if i > 0 && typo_rate > 0 && rand::random_range(0u32..100) < typo_rate {
            // Type the chunk (wrong IME candidate / wrong key)
            axcli_lib::input::type_text(chunk);
            std::thread::sleep(Duration::from_millis(rand::random_range(400..700)));

            // Delete it (one backspace per char)
            for _ in 0..char_count {
                let (kc, fl) = axcli_lib::input::parse_key_combo("Backspace");
                axcli_lib::input::press_key_combo(kc, fl);
                std::thread::sleep(Duration::from_millis(rand::random_range(40..90)));
            }
            std::thread::sleep(Duration::from_millis(rand::random_range(200..500)));
        }

        // Type the correct chunk
        axcli_lib::input::type_text(chunk);

        // Inter-chunk delay (only if not the last chunk)
        if i < chunks.len() - 1 {
            let is_cjk = chunk.chars().any(is_cjk_char);

            let delay = if rand::random_range(0u32..100) < 5 {
                // 5% "thinking" pause
                rand::random_range(2500u64..4000)
            } else if is_cjk {
                // CJK word chunk: simulates pinyin typing + candidate selection
                rand::random_range(1200u64..2200)
            } else if chunk.chars().all(|c| c.is_ascii_alphanumeric()) {
                // ASCII letter/digit: fast touch-typing
                rand::random_range(80u64..200)
            } else {
                // Punctuation / other
                rand::random_range(300u64..600)
            };
            std::thread::sleep(Duration::from_millis(delay));
        }
    }
}

/// Split text into typing chunks:
/// - Consecutive CJK characters → groups of 2-3 (typical Chinese word length)
/// - ASCII characters → individual characters
/// - Punctuation → individual characters
fn split_typing_chunks(text: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut cjk_buf = String::new();

    for ch in text.chars() {
        if is_cjk_char(ch) {
            cjk_buf.push(ch);
            // Flush CJK buffer at word boundary (2-3 chars, with some randomness)
            let word_len = if rand::random_range(0u32..100) < 70 { 2 } else { 3 };
            if cjk_buf.chars().count() >= word_len {
                chunks.push(cjk_buf.clone());
                cjk_buf.clear();
            }
        } else {
            // Flush any pending CJK
            if !cjk_buf.is_empty() {
                chunks.push(cjk_buf.clone());
                cjk_buf.clear();
            }
            // Non-CJK: each character is its own chunk
            chunks.push(ch.to_string());
        }
    }
    // Flush remaining CJK
    if !cjk_buf.is_empty() {
        chunks.push(cjk_buf);
    }

    chunks
}

fn is_cjk_char(ch: char) -> bool {
    let cp = ch as u32;
    // CJK Unified Ideographs + Extension A + Compatibility Ideographs
    (0x4E00..=0x9FFF).contains(&cp)
        || (0x3400..=0x4DBF).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
}

/// Scroll an element down by `pixels` pixels.
/// Moves mouse to element center first (scroll events target mouse position).
pub fn scroll_element_down(locator: &str, pixels: i32) -> Result<()> {
    let node = locate(locator)?;
    let center = element_center(&node, locator)?;
    mouse::move_to(center, Some(100))?;
    std::thread::sleep(std::time::Duration::from_millis(100));
    mouse::scroll_down(center.x, center.y, pixels)?;
    Ok(())
}

/// Chrome window visible content bounds: (x_center, top_y, bottom_y).
pub fn chrome_viewport() -> Result<(f64, f64, f64)> {
    let root = chrome_root()?;
    if let Some(window) = root.locate("window >> nth=0") {
        if let (Some((wx, wy)), Some((ww, wh))) = (window.position(), window.size()) {
            // Title bar (~28px) + toolbar/tabs (~60px) ≈ 88px from window top
            return Ok((wx + ww / 2.0, wy + 88.0, wy + wh));
        }
    }
    Ok((700.0, 100.0, 1400.0)) // fallback
}

/// Scroll on a container at its x-center but with y clamped to the visible
/// viewport.  This ensures the mouse is over the container's column AND
/// within the visible screen (unlike the raw container center which may be
/// off-screen for very tall containers).
pub fn scroll_on_container(container_locator: &str, pixels: i32) -> Result<()> {
    let node = locate(container_locator)?;
    let (px, _py) = node
        .position()
        .ok_or_else(|| anyhow!("cannot get position: {}", container_locator))?;
    let (sx, _sy) = node
        .size()
        .ok_or_else(|| anyhow!("cannot get size: {}", container_locator))?;

    let (_, vp_top, vp_bottom) = chrome_viewport()?;
    let cx = px + sx / 2.0; // container's horizontal center
    let cy = (vp_top + vp_bottom) / 2.0; // viewport's vertical center

    mouse::move_to(
        core_graphics::geometry::CGPoint::new(cx, cy),
        Some(100),
    )?;
    std::thread::sleep(std::time::Duration::from_millis(100));
    mouse::scroll_down(cx, cy, pixels)?;
    Ok(())
}

/// Scroll a container to the top by repeatedly scrolling up.
pub fn scroll_to_top(container_locator: &str) -> Result<()> {
    for _ in 0..15 {
        scroll_on_container(container_locator, -3000)?;
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    Ok(())
}

/// Get the URL of the active Chrome tab via AppleScript
pub fn get_active_tab_url() -> Result<String> {
    let output = Command::new("osascript")
        .args([
            "-e",
            r#"tell application "Google Chrome" to return URL of active tab of front window"#,
        ])
        .output()
        .context("failed to get Chrome tab URL")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Dominant color category detected from an element's screenshot
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DominantColor {
    Red,    // like 已点赞
    Yellow, // favorite 已收藏
    Gray,   // 未操作
    Unknown,
}

/// Capture a screenshot of an element and detect its dominant color.
/// Used to check like/favorite state before clicking.
pub fn detect_element_color(locator: &str) -> Result<DominantColor> {
    use std::ffi::c_void;

    unsafe extern "C" {
        fn CGImageGetWidth(image: *const c_void) -> usize;
        fn CGImageGetHeight(image: *const c_void) -> usize;
        fn CGImageGetBytesPerRow(image: *const c_void) -> usize;
        fn CGImageGetDataProvider(image: *const c_void) -> *const c_void;
        fn CGDataProviderCopyData(provider: *const c_void) -> *const c_void;
        fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
        fn CFDataGetLength(data: *const c_void) -> isize;
        fn CFRelease(cf: *const c_void);
    }

    let node = locate(locator)?;
    let (px, py) = node
        .position()
        .ok_or_else(|| anyhow!("cannot get position: {}", locator))?;
    let (sx, sy) = node
        .size()
        .ok_or_else(|| anyhow!("cannot get size: {}", locator))?;

    // Save screenshot of the element area to a temp file, then read back pixel data.
    // We use axcli_lib's capture which takes objc2 CGRect, so we construct it via
    // the same C types. Since we can't import objc2_core_foundation directly,
    // we transmute from core_graphics types which have identical layout.
    axcli_lib::screenshot::ensure_cg_init();

    // core_graphics::geometry::CGRect has identical C layout to objc2's CGRect
    let cg_rect = core_graphics_types::geometry::CGRect::new(
        &core_graphics_types::geometry::CGPoint::new(px, py),
        &core_graphics_types::geometry::CGSize::new(sx, sy),
    );
    // Safety: CGRect is a plain C struct {origin: {x, y}, size: {w, h}} — layout is identical
    let rect: objc2_core_foundation::CGRect = unsafe { std::mem::transmute(cg_rect) };

    let image = axcli_lib::screenshot::capture(rect)
        .ok_or_else(|| anyhow!("failed to capture screenshot of: {}", locator))?;

    let img_ptr: *const c_void = &*image as *const _ as *const c_void;

    let width = unsafe { CGImageGetWidth(img_ptr) };
    let height = unsafe { CGImageGetHeight(img_ptr) };
    let bytes_per_row = unsafe { CGImageGetBytesPerRow(img_ptr) };

    let provider = unsafe { CGImageGetDataProvider(img_ptr) };
    if provider.is_null() {
        return Ok(DominantColor::Unknown);
    }

    let cf_data = unsafe { CGDataProviderCopyData(provider) };
    if cf_data.is_null() {
        return Ok(DominantColor::Unknown);
    }

    let data_len = unsafe { CFDataGetLength(cf_data) } as usize;
    let data_ptr = unsafe { CFDataGetBytePtr(cf_data) };
    let pixels = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };

    if width == 0 || height == 0 {
        unsafe { CFRelease(cf_data) };
        return Ok(DominantColor::Unknown);
    }

    // Accumulate RGB averages (skip very dark/transparent pixels)
    // macOS screen capture CGImage is BGRA
    let mut r_sum = 0.0_f64;
    let mut g_sum = 0.0_f64;
    let mut b_sum = 0.0_f64;
    let mut count = 0.0_f64;

    for y in 0..height {
        for x in 0..width {
            let offset = y * bytes_per_row + x * 4;
            if offset + 3 >= data_len {
                continue;
            }
            let (b, g, r, a) = (
                pixels[offset] as f64,
                pixels[offset + 1] as f64,
                pixels[offset + 2] as f64,
                pixels[offset + 3] as f64,
            );
            if a < 128.0 || (r + g + b) < 60.0 {
                continue;
            }
            r_sum += r;
            g_sum += g;
            b_sum += b;
            count += 1.0;
        }
    }

    unsafe { CFRelease(cf_data) };

    if count == 0.0 {
        return Ok(DominantColor::Gray);
    }

    let r_avg = r_sum / count;
    let g_avg = g_sum / count;
    let b_avg = b_sum / count;

    // Red: R is dominant and significantly higher than G and B
    // Liked icon mixes with white bg → R:247 G:175 B:180, so use relaxed ratio
    if r_avg > 200.0 && r_avg > g_avg * 1.3 && r_avg > b_avg * 1.3 && (r_avg - g_avg) > 50.0 {
        return Ok(DominantColor::Red);
    }

    // Yellow/Orange/Warm: R > G > B with clear separation
    // Favorited icon blends with white bg → R:251 G:229 B:198, so use R>G>B ordering
    if r_avg > 200.0 && g_avg > 180.0 && r_avg > b_avg * 1.2 && g_avg > b_avg * 1.1 && (r_avg - b_avg) > 40.0 {
        return Ok(DominantColor::Yellow);
    }

    Ok(DominantColor::Gray)
}

/// Press a key combo (e.g., "Command+a", "Enter", "Escape")
pub fn press(key: &str) -> Result<()> {
    let (keycode, flags) = axcli_lib::input::parse_key_combo(key);
    axcli_lib::input::press_key_combo(keycode, flags);
    Ok(())
}

// --- Navigation ---

/// Navigate by setting the active tab's URL via AppleScript (no new tab)
pub fn navigate_open(url: &str) -> Result<()> {
    let script = format!(
        r#"tell application "Google Chrome"
            set URL of active tab of front window to "{}"
        end tell"#,
        url
    );
    Command::new("osascript")
        .args(["-e", &script])
        .output()
        .context("failed to run osascript")?;
    std::thread::sleep(std::time::Duration::from_secs(3));
    Ok(())
}

/// Open a URL in Chrome (creates new tab)
pub fn open_url(url: &str) -> Result<()> {
    Command::new("open")
        .args(["-a", "Google Chrome", url])
        .output()
        .context("failed to open URL")?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    Ok(())
}

// --- Chrome tab management via AppleScript ---

/// Get the active Chrome tab's ID (stable across page navigations)
pub fn get_active_tab_id() -> Result<String> {
    let output = Command::new("osascript")
        .args([
            "-e",
            r#"tell application "Google Chrome" to return id of active tab of front window"#,
        ])
        .output()
        .context("failed to get Chrome tab ID")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Switch to a specific Chrome tab by its ID
pub fn switch_to_tab(tab_id: &str) -> Result<()> {
    let script = format!(
        r#"tell application "Google Chrome"
            repeat with w in windows
                set tabList to tabs of w
                repeat with i from 1 to count of tabList
                    if (id of item i of tabList) as text is "{}" then
                        set active tab index of w to i
                        set index of w to 1
                        activate
                        return true
                    end if
                end repeat
            end repeat
            return false
        end tell"#,
        tab_id
    );
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .context("failed to switch Chrome tab")?;
    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if result != "true" {
        anyhow::bail!("tab {} not found (可能已关闭)", tab_id);
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    Ok(())
}

/// Close a specific Chrome tab by its ID
pub fn close_tab_by_id(tab_id: &str) -> Result<()> {
    let script = format!(
        r#"tell application "Google Chrome"
            repeat with w in windows
                set tabList to tabs of w
                repeat with i from 1 to count of tabList
                    if (id of item i of tabList) as text is "{}" then
                        delete item i of tabList
                        return true
                    end if
                end repeat
            end repeat
            return false
        end tell"#,
        tab_id
    );
    Command::new("osascript")
        .args(["-e", &script])
        .output()
        .context("failed to close tab")?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    Ok(())
}
