use std::path::Path;

use crate::{
    constant::{ACTION_DRAWER_BOTTOM_BAR_TEXT, SCREEN_WIDTH},
    ui::ui_drawer::UIDrawer,
    utils::get_active_color,
    vita2d::{is_button, rgba, vita2d_draw_rect, vita2d_draw_text, SceCtrlButtons},
};

use super::{list_state::ListState, panel::Item};

pub enum MenuAction {
    NewDir,
    Copy,
    Move,
    Rename,
    Delete,
    Zip,
    Unzip,
    Upload,
    ZipUpload,
    Download,
    ChangeAccountId,
}

impl MenuAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            MenuAction::NewDir => "新建文件夹",
            MenuAction::Copy => "复制",
            MenuAction::Move => "移动",
            MenuAction::Rename => "重命名",
            MenuAction::Delete => "删除",
            MenuAction::Zip => "压缩",
            MenuAction::Unzip => "解压",
            MenuAction::Upload => "上传",
            MenuAction::Download => "下载",
            MenuAction::ZipUpload => "压缩并上传",
            MenuAction::ChangeAccountId => "修改 param.sfo 账号为当前账号",
        }
    }
}

pub struct Menu {
    pub list_state: ListState,
    pub actions: Vec<MenuAction>,
    pub drawer: UIDrawer,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            list_state: ListState::new(15),
            actions: vec![],
            drawer: UIDrawer::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        return self.drawer.is_active();
    }

    pub fn is_forces(&self) -> bool {
        return self.drawer.is_forces();
    }

    pub fn get_selected_action(&self) -> Option<&MenuAction> {
        self.actions.get(self.list_state.selected_idx as usize)
    }

    pub fn open(&mut self, item: Option<&Item>, from_path: &str, to_path: &str) {
        self.drawer.open();
        self.actions.clear();
        let is_from_local = !from_path.starts_with("/");
        let is_to_local = !to_path.starts_with("/");
        // new dir
        self.actions.push(MenuAction::NewDir);
        if item.is_none() {
            return;
        }
        let item = item.unwrap();
        [MenuAction::Rename, MenuAction::Delete]
            .into_iter()
            .for_each(|action| {
                self.actions.push(action);
            });
        if is_from_local && is_to_local {
            if to_path != "" {
                self.actions.push(MenuAction::Copy);
                self.actions.push(MenuAction::Move);
            }
            if !item.is_dir && item.name.ends_with(".zip") {
                self.actions.push(MenuAction::Unzip);
            } else {
                self.actions.push(MenuAction::Zip);
            }
            if !item.is_dir
                && item.name == "param.sfo"
                && Path::new(from_path).parent().is_some()
                && Path::new(from_path)
                    .parent()
                    .unwrap()
                    .join("sce_pfs")
                    .exists()
            {
                self.actions.push(MenuAction::ChangeAccountId);
            }
        } else if is_from_local {
            if !item.is_dir && item.name.ends_with(".zip") {
                self.actions.push(MenuAction::Unzip);
                self.actions.push(MenuAction::Upload);
            } else {
                self.actions.push(MenuAction::Zip);
                if !item.is_dir {
                    self.actions.push(MenuAction::Upload);
                }
                self.actions.push(MenuAction::ZipUpload);
            }
            if !item.is_dir && item.name == "param.sfo" {
                self.actions.push(MenuAction::ChangeAccountId);
            }
        } else {
            if !item.is_dir {
                self.actions.push(MenuAction::Download);
            }
        }
    }

    pub fn close(&mut self) {
        self.drawer.close();
    }

    pub fn update(&mut self, buttons: u32) {
        if is_button(buttons, SceCtrlButtons::SceCtrlCross) {
            self.close();
        }

        self.list_state.update(self.actions.len() as i32, buttons);
    }

    pub fn draw_list(&self, left: i32, top: i32) {
        let actions = &self.actions;
        let size = actions.len() as i32;
        let ListState {
            top_row,
            selected_idx,
            display_row,
        } = self.list_state;
        for idx in 0..display_row {
            let i = top_row + idx;
            if i >= size {
                break;
            }
            let x = left + 12;
            let y = top + 22 + 14;
            if i == selected_idx {
                vita2d_draw_rect(
                    x as f32,
                    (y + 30 * idx - 22) as f32,
                    (SCREEN_WIDTH / 2 - 24) as f32,
                    30.0,
                    get_active_color(),
                );
                vita2d_draw_rect(
                    (x + 2) as f32,
                    (y + 2 + 30 * idx - 22) as f32,
                    (SCREEN_WIDTH / 2 - 28) as f32,
                    26.0,
                    rgba(0x18, 0x18, 0x18, 0xff),
                );
            }
            if let Some(action) = actions.get(i as usize) {
                vita2d_draw_text(
                    x + 8,
                    y + 30 * idx,
                    rgba(0xff, 0xff, 0xff, 0xff),
                    1.0,
                    action.as_str(),
                );
            } else {
                println!("actions.get({}) is None", i);
            }
        }
    }

    pub fn draw(&self) {
        if !self.is_active() {
            return;
        }

        self.drawer.draw(ACTION_DRAWER_BOTTOM_BAR_TEXT);
        self.draw_list(self.drawer.get_progress_left() as i32, 0);
    }
}
