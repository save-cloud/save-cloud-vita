use std::time::Instant;

use crate::{
    constant::{ANIME_TIME_300, SCREEN_HEIGHT, SCREEN_WIDTH},
    utils::ease_out_expo,
    vita2d::{
        rgba, vita2d_draw_rect, vita2d_draw_text, vita2d_line, vita2d_text_height,
        vita2d_text_width,
    },
};

pub struct UIDrawer {
    open: bool,
    toggle_at: Instant,
}

impl UIDrawer {
    pub fn new() -> UIDrawer {
        UIDrawer {
            open: false,
            toggle_at: Instant::now() - ANIME_TIME_300,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.toggle_at = Instant::now()
    }

    pub fn close(&mut self) {
        self.open = false;
        self.toggle_at = Instant::now()
    }

    pub fn is_active(&self) -> bool {
        return self.open || Instant::now().duration_since(self.toggle_at) <= ANIME_TIME_300;
    }

    pub fn get_progress_left(&self) -> f32 {
        let (start, end) = if self.open {
            (SCREEN_WIDTH as f32, (SCREEN_WIDTH / 2) as f32)
        } else {
            ((SCREEN_WIDTH / 2) as f32, SCREEN_WIDTH as f32)
        };
        ease_out_expo(
            Instant::now().duration_since(self.toggle_at),
            ANIME_TIME_300,
            start,
            end,
        )
    }

    pub fn draw_bottom_bar(&self, x: f32, text: &str) {
        vita2d_line(
            x + 12.0,
            (SCREEN_HEIGHT - 58) as f32,
            (SCREEN_WIDTH - 12) as f32,
            (SCREEN_HEIGHT - 58) as f32,
            rgba(0x99, 0x99, 0x99, 0xff),
        );
        vita2d_draw_text(
            x as i32 + SCREEN_WIDTH / 2 - 12 - vita2d_text_width(1.0, text),
            SCREEN_HEIGHT - (58 / 2) + vita2d_text_height(1.0, text) / 2,
            rgba(0xff, 0xff, 0xff, 0xff),
            1.0,
            text,
        )
    }

    pub fn draw(&self, text: &str) {
        if !self.is_active() {
            return;
        }
        let x = self.get_progress_left();
        vita2d_draw_rect(
            x,
            0.0,
            (SCREEN_WIDTH / 2) as f32,
            SCREEN_HEIGHT as f32,
            rgba(0x18, 0x18, 0x18, 0xff),
        );
        self.draw_bottom_bar(x, text)
    }

    pub fn is_forces(&self) -> bool {
        self.open
    }
}
