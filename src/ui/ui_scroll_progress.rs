use crate::vita2d::{is_button, SceCtrlButtons};

pub struct ScrollProgress {
    pub progress: f32,
    pub delay: f32,
    pub max: f32,
}

impl ScrollProgress {
    pub fn new(delay: f32, max: f32) -> Self {
        Self {
            progress: 0.0,
            delay,
            max,
        }
    }

    pub fn update(&mut self, buttons: u32) {
        if is_button(buttons, SceCtrlButtons::SceCtrlUp)
            || is_button(buttons, SceCtrlButtons::SceCtrlDown)
        {
            self.progress = -self.delay;
        } else {
            self.progress += 1.0;
            if self.progress > self.max + self.delay {
                self.progress = -self.delay;
            }
        }
    }

    pub fn progress(&self) -> f32 {
        if self.progress < 0.0 {
            0.0
        } else if self.progress > self.max {
            1.0
        } else {
            self.progress / self.max
        }
    }
}
