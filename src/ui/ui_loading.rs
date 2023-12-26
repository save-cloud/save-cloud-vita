use std::{
    sync::{OnceLock, RwLock},
    time::Instant,
};

use crate::{
    constant::{ANIME_TIME_300, DIALOG_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH},
    utils::{current_time, ease_out_expo},
    vita2d::{rgba, vita2d_draw_rect, vita2d_draw_text, vita2d_text_height, vita2d_text_width},
};

static LOADING: OnceLock<RwLock<Loading>> = OnceLock::new();

pub struct Loading {
    open: bool,
    title: Option<String>,
    desc: Option<String>,
    toggle_at: Instant,
}

pub fn draw_loading(x: f32, y: f32, size: f32) {
    let idx = ((current_time() % 1000) / 250) as i32;
    for i in 0..4 {
        vita2d_draw_rect(
            x + (if i > 0 && i < 3 { 1 } else { 0 }) as f32 * size,
            y + (i / 2) as f32 * size,
            size,
            size,
            rgba(
                0x99,
                0x99,
                0x99,
                if i == idx {
                    0x40
                } else {
                    let pre = if idx - 1 >= 0 { idx - 1 } else { 3 };
                    let pre1 = if pre - 1 >= 0 { pre - 1 } else { 3 };
                    if i == pre {
                        0xff
                    } else if i == pre1 {
                        0xa0
                    } else {
                        0x60
                    }
                },
            ),
        );
    }
}

impl Loading {
    fn get() -> &'static RwLock<Loading> {
        LOADING.get_or_init(|| {
            RwLock::new(Loading {
                open: false,
                title: None,
                desc: None,
                toggle_at: Instant::now() - ANIME_TIME_300,
            })
        })
    }

    pub fn title() -> Option<String> {
        if let Ok(lock) = Self::get().try_read() {
            return lock.title.clone();
        }
        None
    }

    pub fn desc() -> Option<String> {
        if let Ok(lock) = Self::get().try_read() {
            return lock.desc.clone();
        }
        None
    }

    pub fn is_active() -> bool {
        Self::get().read().expect("read loading status").open
            || Instant::now().duration_since(Self::toggle_at()) < ANIME_TIME_300
    }

    pub fn is_pending() -> bool {
        Self::get().read().expect("read loading status").open
    }

    pub fn toggle_at() -> Instant {
        Self::get().read().expect("read loading status").toggle_at
    }

    pub fn show() {
        if Self::is_pending() {
            return;
        }
        let mut s = Self::get().write().expect("write loading status");
        s.toggle_at = Instant::now();
        s.open = true;
        s.title = None;
        s.desc = None;
    }

    pub fn hide() {
        if !Self::is_pending() {
            return;
        }
        let mut s = Self::get().write().expect("write loading status");
        s.toggle_at = Instant::now();
        s.open = false;
    }

    pub fn get_progress_top() -> f32 {
        let (start, end) = if Self::is_pending() {
            (SCREEN_HEIGHT as f32, (SCREEN_HEIGHT - 74) as f32)
        } else {
            ((SCREEN_HEIGHT - 74) as f32, SCREEN_HEIGHT as f32)
        };
        ease_out_expo(
            Instant::now().duration_since(Self::toggle_at()),
            ANIME_TIME_300,
            start,
            end,
        )
    }

    pub fn notify_title(title: String) {
        let mut s = Self::get().write().expect("write loading status");
        s.title = Some(title);
    }

    pub fn notify_desc(desc: String) {
        let mut s = Self::get().write().expect("write loading status");
        s.desc = Some(desc);
    }

    pub fn draw_rect() {
        draw_loading(14.0, Loading::get_progress_top(), 30.0);
    }

    pub fn draw() {
        if !Self::is_active() {
            return;
        }
        Self::draw_rect();

        if !Self::is_pending() {
            return;
        }

        if let Some(desc) = Self::desc() {
            // bg
            vita2d_draw_rect(
                ((SCREEN_WIDTH - DIALOG_WIDTH) / 2) as f32,
                ((SCREEN_HEIGHT - 120) / 2) as f32,
                DIALOG_WIDTH as f32,
                120.0,
                rgba(0x44, 0x44, 0x44, 0xff),
            );
            if let Some(title) = Self::title() {
                let x = (SCREEN_WIDTH - DIALOG_WIDTH) / 2 + 14;
                vita2d_draw_text(
                    x,
                    (SCREEN_HEIGHT - 120) / 2 + 14 + vita2d_text_height(1.0, &title),
                    rgba(0xff, 0xff, 0xff, 0xff),
                    1.0,
                    &title,
                );
            }
            vita2d_draw_text(
                (SCREEN_WIDTH - vita2d_text_width(1.0, &desc)) / 2,
                (SCREEN_HEIGHT + vita2d_text_height(1.0, &desc)) / 2,
                rgba(0xff, 0xff, 0xff, 0xff),
                1.0,
                &desc,
            );
        }
    }
}
