use std::{
    sync::{OnceLock, RwLock},
    time::{Duration, Instant},
};

use crate::{
    constant::{ANIME_TIME_300, SCREEN_HEIGHT},
    utils::ease_out_expo,
    vita2d::{rgba, vita2d_draw_rect, vita2d_draw_text, vita2d_text_width},
};

static TOAST: OnceLock<RwLock<Toast>> = OnceLock::new();

pub struct Toast {
    text: Option<String>,
    open: bool,
    toggle_at: Instant,
}

impl Toast {
    fn get() -> &'static RwLock<Toast> {
        TOAST.get_or_init(|| {
            RwLock::new(Toast {
                text: None,
                open: false,
                toggle_at: Instant::now() - ANIME_TIME_300,
            })
        })
    }

    pub fn is_active() -> bool {
        Self::get().read().expect("read toast status").open
            || Instant::now().duration_since(Self::toggle_at()) < ANIME_TIME_300
    }

    pub fn is_pending() -> bool {
        Self::get().read().expect("read toast status").open
    }

    pub fn toggle_at() -> Instant {
        Self::get().read().expect("read toast status").toggle_at
    }

    pub fn show(text: String) {
        let mut s = Self::get().write().expect("write toast status");
        s.text = Some(text);
        s.toggle_at = Instant::now();
        s.open = true;
    }

    pub fn hide() {
        if !Self::is_pending() {
            return;
        }
        let mut s = Self::get().write().expect("write toast status");
        s.toggle_at = Instant::now();
        s.open = false;
    }

    pub fn text() -> Option<String> {
        if let Some(text) = &Self::get().read().expect("read toast status").text {
            return Some(text.to_owned());
        }

        None
    }

    pub fn get_progress_top() -> f32 {
        let (start, end) = if Self::is_pending() {
            ((SCREEN_HEIGHT + 26) as f32, (SCREEN_HEIGHT - 80) as f32)
        } else {
            ((SCREEN_HEIGHT - 80) as f32, (SCREEN_HEIGHT + 26) as f32)
        };
        ease_out_expo(
            Instant::now().duration_since(Self::toggle_at()),
            ANIME_TIME_300,
            start,
            end,
        )
    }

    pub fn draw() {
        if !Self::is_active() {
            return;
        }
        if Self::is_pending() && (Instant::now() - Self::toggle_at() > Duration::from_millis(3000))
        {
            Self::hide();
        }
        let top = Self::get_progress_top();
        if let Some(text) = Self::text() {
            vita2d_draw_rect(
                14.0,
                top - 26.0,
                vita2d_text_width(1.2, &text) as f32 + 28.0,
                36.0,
                rgba(0x66, 0x66, 0x66, 0xff),
            );
            vita2d_draw_text(
                14 + 14,
                top as i32,
                rgba(0xff, 0xff, 0xff, 0xff),
                1.2,
                &text,
            )
        }
    }
}
