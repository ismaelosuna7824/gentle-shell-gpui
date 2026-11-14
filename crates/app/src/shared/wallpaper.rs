//! The active theme's background, painted behind the panes: soft aurora
//! blobs, falling petals, twinkling stars, rain or a turning gradient. The
//! panes are glass, so it shows through them at the window's transparency.
//!
//! Paint-only (no input, no layout). It moves on the shared live clock at
//! the slow live rate, only while the window is focused, and holds still
//! with reduced motion or a speed of 0.

use gpui_kit::*;

use crate::shared::themes::{Themes, Wallpaper, WallpaperKind};
use crate::shared::ui;

/// The background layer for the window, or `None` when the active theme has
/// none.
pub fn layer(window: &Window, cx: &App) -> Option<AnyElement> {
    let wallpaper = cx.global::<Themes>().active_theme().background.clone()?;
    let moving = wallpaper.speed > 0. && window.is_window_active() && !cx.reduce_motion();
    // A still background is drawn at a fixed moment so it doesn't jump.
    let t = if moving { ui::live_clock() } else { 40. } * wallpaper.speed.max(0.25);
    Some(
        canvas(
            move |_, window, cx| {
                if moving {
                    ui::request_slow_frame(window, cx);
                }
            },
            move |bounds, _, window, _| paint(&wallpaper, bounds, t, window),
        )
        .absolute()
        .inset_0()
        .into_any_element(),
    )
}

/// A stable pseudo-random number in 0..1 for item `i` and `salt`.
fn rand(i: usize, salt: u32) -> f32 {
    let mut x = (i as u32).wrapping_mul(0x9E37_79B9) ^ salt.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65536.
}

fn tint(hex: u32, alpha: f32) -> Hsla {
    let color: Hsla = rgb(hex).into();
    color.opacity(alpha.clamp(0., 1.))
}

fn dot(window: &mut Window, x: f32, y: f32, w: f32, h: f32, color: Hsla) {
    let bounds = Bounds::new(point(px(x - w / 2.), px(y - h / 2.)), size(px(w), px(h)));
    window.paint_quad(fill(bounds, color).corner_radii(px(w.min(h) / 2.)));
}

fn paint(wallpaper: &Wallpaper, bounds: Bounds<Pixels>, t: f32, window: &mut Window) {
    let (x0, y0) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let colors = &wallpaper.colors;
    let color = |i: usize| colors[i % colors.len()];
    let strength = wallpaper.intensity;
    match wallpaper.kind {
        WallpaperKind::Aurora => {
            // Each blob is a stack of fading discs, wandering on a slow
            // Lissajous path.
            const RINGS: usize = 14;
            let blobs = colors.len().clamp(3, 4);
            let span = w.max(h);
            for k in 0..blobs {
                let kf = k as f32;
                let cx = x0 + w * (0.5 + 0.38 * (t * 0.05 * (1. + kf * 0.31) + kf * 2.1).sin());
                let cy = y0 + h * (0.45 + 0.32 * (t * 0.037 * (1. + kf * 0.23) + kf * 1.3).cos());
                let radius = span * (0.28 + 0.12 * rand(k, 1));
                for ring in 0..RINGS {
                    let r = radius * (1. - ring as f32 / RINGS as f32);
                    dot(
                        window,
                        cx,
                        cy,
                        r * 2.,
                        r * 1.6,
                        tint(color(k), strength * 0.045),
                    );
                }
            }
        }
        WallpaperKind::Petals => {
            let count = (18. + 30. * strength) as usize;
            for i in 0..count {
                let fall = 16. + 24. * rand(i, 2);
                let travel = h + 40.;
                let y = y0 + (t * fall + rand(i, 3) * travel) % travel - 20.;
                let sway = 22. * (t * (0.5 + 0.6 * rand(i, 4)) + i as f32).sin();
                let x = x0 + rand(i, 5) * w + sway;
                // Turning: the petal's width breathes as it tumbles.
                let turn = (t * (0.7 + rand(i, 6)) + i as f32 * 1.7).sin().abs();
                let pw = (6. + 7. * rand(i, 7)) * (0.45 + 0.55 * turn);
                let ph = 5. + 4. * rand(i, 8);
                let alpha = strength * (0.35 + 0.45 * rand(i, 9));
                dot(window, x, y, pw, ph, tint(color(i), alpha));
            }
        }
        WallpaperKind::Stars => {
            let count = (50. + 90. * strength) as usize;
            for i in 0..count {
                let x = x0 + (rand(i, 10) * w + t * (1. + 2. * rand(i, 11))) % w;
                let y = y0 + rand(i, 12) * h;
                let size = 1. + 1.8 * rand(i, 13);
                let twinkle = 0.5 + 0.5 * (t * (0.6 + 1.4 * rand(i, 14)) + i as f32).sin();
                let alpha = strength * (0.25 + 0.75 * twinkle);
                // Most stars are the first color; a few take the others.
                let tone = if rand(i, 15) < 0.75 { 0 } else { 1 + i };
                dot(window, x, y, size, size, tint(color(tone), alpha));
            }
        }
        WallpaperKind::Rain => {
            let count = (30. + 60. * strength) as usize;
            for i in 0..count {
                let len = 14. + 22. * rand(i, 16);
                let speed = 220. + 220. * rand(i, 17);
                let travel = h + len;
                let y = y0 + (t * speed + rand(i, 18) * travel) % travel - len / 2.;
                let x = x0 + rand(i, 19) * w;
                let alpha = strength * (0.2 + 0.4 * rand(i, 20));
                let bounds = Bounds::new(point(px(x), px(y - len / 2.)), size(px(1.2), px(len)));
                window.paint_quad(fill(bounds, tint(color(i), alpha)).corner_radii(px(0.6)));
            }
        }
        WallpaperKind::Gradient => {
            let angle = (t * 4.) % 360.;
            let from = linear_color_stop(tint(color(0), strength * 0.45), 0.);
            let to = linear_color_stop(tint(color(1), strength * 0.45), 1.);
            window.paint_quad(fill(bounds, linear_gradient(angle, from, to)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rand;

    #[test]
    fn rand_is_stable_and_in_range() {
        for i in 0..500 {
            let r = rand(i, 7);
            assert!((0. ..1.).contains(&r));
            assert_eq!(r, rand(i, 7));
        }
        // Different salts spread the same item.
        assert_ne!(rand(3, 1), rand(3, 2));
    }
}
