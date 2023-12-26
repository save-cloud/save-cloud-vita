use std::sync::{Arc, RwLock};

use crate::{
    app::AppData,
    constant::SCREEN_WIDTH,
    ui::ui_scroll_progress::ScrollProgress,
    utils::get_active_color,
    vita2d::{
        is_button, rgba, vita2d_draw_rect, vita2d_draw_text, vita2d_draw_texture_scale,
        vita2d_set_clip, vita2d_text_width, vita2d_unset_clip, SceCtrlButtons, Vita2dTexture,
    },
};

use super::{
    action::{cloud::CloudAction, local::LocalAction, Action},
    list_state::ListState,
};

pub struct Item {
    pub name: String,
    pub is_dir: bool,
    pub fs_id: Option<u64>,
}

impl Item {
    pub fn new(is_dir: bool, name: String, fs_id: Option<u64>) -> Item {
        Item {
            name,
            is_dir,
            fs_id,
        }
    }
}

pub struct Dir {
    pub name: String,
    pub items: Vec<Item>,
    pub state: ListState,
}

impl Dir {
    pub fn new(name: String, items: Vec<Item>) -> Dir {
        Dir {
            name,
            items,
            state: ListState::new(12),
        }
    }

    pub fn add_item(&mut self, is_dir: bool, name: String, fs_id: Option<u64>) {
        self.items.push(Item::new(is_dir, name, fs_id));
    }

    pub fn current_item(&self) -> Option<&Item> {
        self.items.get(self.state.selected_idx as usize)
    }
}

pub enum DirPendingAction {
    Enter,
    Refresh,
}

pub struct DirPending {
    pub action: DirPendingAction,
    pub dir: Dir,
}

pub struct Panel {
    pub left: i32,
    pub dirs: Vec<Dir>,
    pub action: Box<dyn Action>,
    pub dir_pending_to_enter: Arc<RwLock<Option<DirPending>>>,
    // 0-120
    pub scroll_progress: ScrollProgress,
}

impl Panel {
    pub fn new_local(left: i32) -> Panel {
        Panel {
            left,
            dirs: vec![],
            action: Box::new(LocalAction::new()),
            dir_pending_to_enter: Arc::new(RwLock::new(None)),
            scroll_progress: ScrollProgress::new(40.0, 110.0),
        }
    }

    pub fn new_cloud(left: i32) -> Panel {
        Panel {
            left,
            dirs: vec![],
            action: Box::new(CloudAction::new()),
            dir_pending_to_enter: Arc::new(RwLock::new(None)),
            scroll_progress: ScrollProgress::new(40.0, 110.0),
        }
    }

    pub fn init(&mut self) {
        if self.is_pending() {
            return;
        }
        if let Ok(mut dir) = self.dir_pending_to_enter.try_write() {
            if let Some(mut cmd) = dir.take() {
                match cmd.action {
                    DirPendingAction::Refresh => {
                        let old_dir = self.dirs.pop();
                        cmd.dir.state = old_dir.unwrap().state;
                    }
                    _ => {}
                }
                self.dirs.push(cmd.dir);
            }
        }
        self.action.init(&mut self.dirs, &self.dir_pending_to_enter);
    }

    pub fn is_pending(&self) -> bool {
        Arc::strong_count(&self.dir_pending_to_enter) > 1
    }

    pub fn current_item(&self) -> Option<&Item> {
        if let Some(dir) = self.current_dir() {
            return dir.current_item();
        }
        None
    }

    pub fn current_dir(&self) -> Option<&Dir> {
        self.dirs.last()
    }

    // end with /
    pub fn current_dir_path(&self) -> String {
        self.dirs
            .iter()
            .map(|dir| {
                // psv device end with :
                if dir.name == "" || dir.name == "/" {
                    dir.name.clone()
                } else {
                    format!("{}/", dir.name)
                }
            })
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn is_forces(&self) -> bool {
        self.is_pending()
    }

    pub fn refresh_current_dir(&mut self) {
        self.action.do_action(
            &self.current_dir_path(),
            "",
            DirPendingAction::Refresh,
            &self.dir_pending_to_enter,
        );
    }

    pub fn update(&mut self, _app_data: &mut AppData, buttons: u32) {
        self.scroll_progress.update(buttons);

        if self.is_pending() {
            return;
        }
        if is_button(buttons, SceCtrlButtons::SceCtrlCircle) {
            if let Some(item) = self.current_item() {
                if item.is_dir {
                    self.action.do_action(
                        &self.current_dir_path(),
                        &item.name,
                        DirPendingAction::Enter,
                        &self.dir_pending_to_enter,
                    );
                }
            }
        } else if is_button(buttons, SceCtrlButtons::SceCtrlCross) {
            self.action.pop_dir(&mut self.dirs);
        } else if let Some(dir) = self.dirs.last_mut() {
            dir.state.update(dir.items.len() as i32, buttons);
        }
    }

    pub fn draw(&self, is_active: bool, no_data_tex: &Option<Vita2dTexture>) {
        if let Some(dir) = self.current_dir() {
            let size = dir.items.len() as i32;
            if size == 0 && !self.is_pending() {
                if let Some(tex) = no_data_tex {
                    vita2d_draw_texture_scale(
                        tex,
                        (self.left + (SCREEN_WIDTH / 2 - 24 - 68) / 2) as f32,
                        260.0,
                        0.54,
                        0.54,
                    );
                }
            } else {
                let ListState {
                    top_row,
                    display_row,
                    selected_idx,
                } = &dir.state;
                for idx in 0..*display_row {
                    let i = top_row + idx;
                    if i > size {
                        break;
                    }
                    if let Some(item) = dir.items.get(i as usize) {
                        let x = self.left;
                        let y = 129;
                        if is_active {
                            if i == *selected_idx {
                                vita2d_draw_rect(
                                    x as f32,
                                    (y + 30 * idx - 21) as f32,
                                    (SCREEN_WIDTH / 2 - 24) as f32,
                                    30.0,
                                    get_active_color(),
                                );
                                vita2d_draw_rect(
                                    (x + 2) as f32,
                                    (y + 2 + 30 * idx - 21) as f32,
                                    (SCREEN_WIDTH / 2 - 28) as f32,
                                    26.0,
                                    rgba(0x2c, 0x2d, 0x31, 0xff),
                                );
                            }
                        }

                        let mut x = x + 8;
                        let text_width = vita2d_text_width(1.0, &item.name);
                        if text_width > SCREEN_WIDTH / 2 - 40 {
                            vita2d_set_clip(
                                x,
                                y + 2 + 30 * idx - 21,
                                x + (SCREEN_WIDTH / 2 - 40),
                                (y + 2 + 30 * idx - 21) + 26,
                            );
                            if i == *selected_idx {
                                x = x
                                    - ((text_width - (SCREEN_WIDTH / 2 - 40)) as f32
                                        * self.scroll_progress.progress())
                                        as i32;
                            }
                        }
                        vita2d_draw_text(
                            x,
                            y + 30 * idx,
                            if item.is_dir {
                                rgba(0x00, 0xb4, 0xd8, 0xff)
                            } else {
                                rgba(0xee, 0xee, 0xee, 0xff)
                            },
                            1.0,
                            &item.name,
                        );
                        if text_width > SCREEN_WIDTH / 2 - 40 {
                            vita2d_unset_clip();
                        }
                    }
                }
            }
        }
    }
}
