//! Human-like mouse movement using CoreGraphics.
//!
//! Simulates natural hand movement:
//! - Fast start → cruise → sharp braking near target
//! - Slight overshoot past target, then corrective snap-back
//! - Micro jitter throughout the path

use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::thread;
use std::time::Duration;

fn rand_f64(min: f64, max: f64) -> f64 {
    rand::random_range(min..max)
}

/// Get current mouse cursor position
fn get_mouse_position() -> CGPoint {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).unwrap();
    let event = CGEvent::new(source).unwrap();
    event.location()
}

/// Quadratic Bezier curve point at parameter t (0..1)
fn bezier_point(p0: CGPoint, p1: CGPoint, p2: CGPoint, t: f64) -> CGPoint {
    let inv = 1.0 - t;
    CGPoint::new(
        inv * inv * p0.x + 2.0 * inv * t * p1.x + t * t * p2.x,
        inv * inv * p0.y + 2.0 * inv * t * p1.y + t * t * p2.y,
    )
}

/// Human-like easing: fast start → cruise → sharp brake
///
/// Covers ~85% of distance in first 60% of time, then brakes hard.
fn ease_fast_brake(t: f64) -> f64 {
    // Quadratic ease-out: fast start, gradual deceleration
    1.0 - (1.0 - t).powi(2)
}

/// Post a mouse-moved CGEvent at the given point
fn post_mouse_move(source: &CGEventSource, point: CGPoint) -> anyhow::Result<()> {
    let event = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::MouseMoved,
        point,
        CGMouseButton::Left,
    )
    .map_err(|_| anyhow::anyhow!("failed to create mouse event"))?;
    event.post(CGEventTapLocation::HID);
    Ok(())
}

/// Move mouse from current position to target with human-like trajectory.
///
/// Behavior:
/// 1. Fast acceleration → cruise → sharp braking (ease-out)
/// 2. Slight overshoot past target (5-15px)
/// 3. Brief pause ("oh, went too far")
/// 4. Corrective snap-back to exact target
pub fn move_to(target: CGPoint, duration_ms: Option<u64>) -> anyhow::Result<()> {
    let start = get_mouse_position();

    let dx = target.x - start.x;
    let dy = target.y - start.y;
    let distance = (dx * dx + dy * dy).sqrt();

    if distance < 3.0 {
        return Ok(());
    }

    // --- Phase 1: Main movement (start → overshoot point) ---

    // Calculate overshoot point (skip for very short distances)
    let do_overshoot = distance > 30.0;
    let move_angle = dy.atan2(dx);
    let overshoot_target = if do_overshoot {
        let overshoot_dist = rand_f64(5.0, 15.0);
        CGPoint::new(
            target.x + overshoot_dist * move_angle.cos(),
            target.y + overshoot_dist * move_angle.sin(),
        )
    } else {
        target
    };

    let duration = duration_ms.unwrap_or_else(|| {
        let base = 250.0 + distance * 0.4;
        (base + rand_f64(-50.0, 50.0)).clamp(200.0, 700.0) as u64
    });

    // Random control point for Bezier curve (perpendicular offset for arc)
    let mid_x = (start.x + overshoot_target.x) / 2.0;
    let mid_y = (start.y + overshoot_target.y) / 2.0;
    let perp_offset = rand_f64(-distance * 0.12, distance * 0.12);
    let perp_angle = move_angle + std::f64::consts::FRAC_PI_2;
    let control = CGPoint::new(
        mid_x + perp_offset * perp_angle.cos(),
        mid_y + perp_offset * perp_angle.sin(),
    );

    let steps = (duration as f64 / 12.0).clamp(10.0, 80.0) as usize;
    let step_delay = Duration::from_micros((duration * 1000) / steps as u64);

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| anyhow::anyhow!("failed to create CGEventSource"))?;

    for i in 1..=steps {
        let t = ease_fast_brake(i as f64 / steps as f64);
        let mut point = bezier_point(start, control, overshoot_target, t);

        // Micro jitter (±0.5px) except at final position
        if i < steps {
            point.x += rand_f64(-0.5, 0.5);
            point.y += rand_f64(-0.5, 0.5);
        }

        post_mouse_move(&source, point)?;
        thread::sleep(step_delay);
    }

    // --- Phase 2: Corrective snap-back (overshoot → target) ---

    if do_overshoot {
        // Brief pause — "realized I overshot"
        thread::sleep(Duration::from_millis(rand_f64(30.0, 80.0) as u64));

        let correction_steps = 5_usize;
        let correction_step_delay =
            Duration::from_millis(rand_f64(15.0, 35.0) as u64);

        for i in 1..=correction_steps {
            let t = i as f64 / correction_steps as f64;
            let point = CGPoint::new(
                overshoot_target.x + (target.x - overshoot_target.x) * t,
                overshoot_target.y + (target.y - overshoot_target.y) * t,
            );
            post_mouse_move(&source, point)?;
            thread::sleep(correction_step_delay);
        }
    }

    Ok(())
}

/// Click at the current mouse position
pub fn click_at_current() -> anyhow::Result<()> {
    let pos = get_mouse_position();
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| anyhow::anyhow!("failed to create CGEventSource"))?;

    let down = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::LeftMouseDown,
        pos,
        CGMouseButton::Left,
    )
    .map_err(|_| anyhow::anyhow!("failed to create mouse down event"))?;
    down.post(CGEventTapLocation::HID);

    // Random hold: 50-120ms
    thread::sleep(Duration::from_millis(rand::random_range(50..120)));

    let up = CGEvent::new_mouse_event(
        source,
        CGEventType::LeftMouseUp,
        pos,
        CGMouseButton::Left,
    )
    .map_err(|_| anyhow::anyhow!("failed to create mouse up event"))?;
    up.post(CGEventTapLocation::HID);

    Ok(())
}

/// Scroll down at screen position (x, y) by `pixels` pixels.
pub fn scroll_down(x: f64, y: f64, pixels: i32) -> anyhow::Result<()> {
    axcli_lib::input::scroll_wheel(x, y, 0, -pixels);
    Ok(())
}

