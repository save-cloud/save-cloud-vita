use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock, RwLockReadGuard,
    },
};

use log::error;

use crate::{
    api::Api,
    constant::{GAME_SAVE_CLOUD_DIR, HOME_PAGE_URL, LIST_NAME_WIDTH, SCREEN_WIDTH},
    ime::{get_current_format_time, show_keyboard},
    tai::{mount_pfs, Title},
    ui::{
        ui_cloud::list_state::ListState, ui_dialog::UIDialog, ui_list::UIList, ui_loading::Loading,
        ui_scroll_progress::ScrollProgress, ui_toast::Toast,
    },
    utils::{
        backup_game_save, get_active_color, get_game_local_backup_dir, get_local_game_saves,
        normalize_path, restore_game_save,
    },
    vita2d::{
        is_button, rgba, vita2d_draw_rect, vita2d_draw_text, vita2d_set_clip, vita2d_text_width,
        vita2d_unset_clip, SceCtrlButtons,
    },
};

use super::DISPLAY_ROW;

pub struct SaveListLocal {
    pending: Arc<AtomicBool>,
    list_state: ListState,
    local_dir: String,
    cloud_dir: Arc<RwLock<String>>,
    title_id: String,
    items: Arc<RwLock<Vec<String>>>,
    new_backup_text: &'static str,
    scroll_progress: ScrollProgress,
}

impl SaveListLocal {
    pub fn new(new_back: &'static str, title: &Title) -> SaveListLocal {
        SaveListLocal {
            list_state: ListState::new(DISPLAY_ROW),
            pending: Arc::new(AtomicBool::new(false)),
            local_dir: get_game_local_backup_dir(&title.title_id(), &title.name()),
            cloud_dir: Arc::new(RwLock::new(
                format!(
                    "{}/{} {}",
                    GAME_SAVE_CLOUD_DIR,
                    title.title_id(),
                    normalize_path(title.name().trim())
                )
                .trim()
                .to_string(),
            )),
            title_id: title.title_id().to_string(),
            items: Arc::new(RwLock::new(vec![])),
            new_backup_text: new_back,
            scroll_progress: ScrollProgress::new(40.0, 100.0),
        }
    }

    pub fn get_items(&self) -> RwLockReadGuard<Vec<String>> {
        self.items.read().expect("read game saves")
    }

    fn local_dir(&self) -> String {
        self.local_dir.to_string()
    }

    fn cloud_dir(&self) -> String {
        self.cloud_dir
            .read()
            .expect("read cloud save dir")
            .to_string()
    }

    fn upload_backup(&self) {
        // upload
        let idx = self.list_state.selected_idx - 1;
        if idx >= 0 {
            let backup_name = self.get_items().get(idx as usize).unwrap().to_owned();
            if UIDialog::present(&format!("上传备份：{}？", backup_name)) {
                let local_backup_path = format!("{}/{}", self.local_dir(), backup_name);
                let title_id = self.title_id.to_string();
                let cloud_dir = self.cloud_dir();
                let pending = Arc::clone(&self.pending);
                pending.store(true, Ordering::Relaxed);
                Loading::show();
                tokio::spawn(async move {
                    Loading::notify_title("正在上传存档".to_string());
                    Loading::notify_desc(backup_name.clone());
                    let (game_save_dir, list) = Api::fetch_save_cloud_list(&title_id, false);
                    if !(list.is_some()
                        && list
                            .unwrap()
                            .iter()
                            .find(|&item| item.name == backup_name)
                            .is_some())
                    {
                        let game_save_dir = if game_save_dir.is_some() {
                            game_save_dir.unwrap()
                        } else {
                            cloud_dir
                        };
                        match Api::upload_to_cloud(
                            &game_save_dir,
                            &backup_name,
                            &local_backup_path,
                            false,
                        ) {
                            Ok(_) => {
                                Toast::show("备份上传完成！".to_string());
                            }
                            Err(err) => {
                                error!("upload {} to cloud failed: {:?}", local_backup_path, err);
                                Toast::show(format!("备份上传失败：{}", err));
                            }
                        }
                    } else {
                        Toast::show("同名云备份已存在！".to_string());
                    }
                    Loading::hide();
                    pending.store(false, Ordering::Relaxed);
                });
            }
        }
    }
}

impl UIList for SaveListLocal {
    fn init(&mut self) {
        let local_dir = self.local_dir();
        if !Path::new(&local_dir).exists() {
            return;
        }
        let items = Arc::clone(&self.items);
        tokio::spawn(async move {
            get_local_game_saves(local_dir, items);
        });
    }

    fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }

    fn do_restore_game_save(&self, game_save_dir: &Option<String>, backup_name: &str) {
        match &game_save_dir {
            Some(game_save_dir) => {
                let game_save_dir = game_save_dir.to_string();
                let backup_name = format!("{}/{}", self.local_dir, backup_name);
                let local_dir = self.local_dir();
                let items = Arc::clone(&self.items);
                let pending = Arc::clone(&self.pending);
                pending.store(true, Ordering::Relaxed);
                Loading::show();
                mount_pfs(&game_save_dir);
                tokio::spawn(async move {
                    Loading::notify_title("正在恢复存档".to_string());
                    match restore_game_save(&backup_name, &game_save_dir) {
                        Ok(_) => {
                            get_local_game_saves(local_dir, items);
                            Toast::show("存档恢复完成！".to_string());
                        }
                        Err(err) => {
                            error!(
                                "extract zip {} to {} failed: {:?}",
                                backup_name, game_save_dir, err
                            );
                            Toast::show(format!("存档恢复失败：{}", err));
                        }
                    }
                    Loading::hide();
                    pending.store(false, Ordering::Relaxed);
                });
            }
            None => {
                Toast::show("没有找到游戏存档，请先运行游戏！".to_string());
            }
        }
    }

    fn do_backup_game_save(&self, game_save_dir: &Option<String>, input: Option<String>) {
        match &game_save_dir {
            Some(game_save_dir) => {
                let game_save_dir = game_save_dir.to_string();
                let backup_name = match &input {
                    Some(input) => format!("{}/{}", self.local_dir, input),
                    None => {
                        let input = show_keyboard(&get_current_format_time());
                        if input.len() > 0 {
                            format!("{}/{}.zip", self.local_dir, input)
                        } else {
                            "".to_string()
                        }
                    }
                };
                if backup_name.len() > 0 {
                    let is_overwrite = input.is_some();
                    let local_dir = self.local_dir();
                    let items = Arc::clone(&self.items);
                    let pending = Arc::clone(&self.pending);
                    pending.store(true, Ordering::Relaxed);
                    Loading::show();
                    mount_pfs(&game_save_dir);
                    tokio::spawn(async move {
                        Loading::notify_title("正在备份".to_string());
                        match backup_game_save(&game_save_dir, &backup_name) {
                            Ok(_) => {
                                // update save list
                                get_local_game_saves(local_dir, items);
                                Toast::show(if !is_overwrite {
                                    "备份完成！".to_string()
                                } else {
                                    "备份覆盖完成！".to_string()
                                });
                            }
                            Err(err) => {
                                error!(
                                    "zip {} to {} failed: {:?}",
                                    game_save_dir, backup_name, err
                                );
                                Toast::show(format!("备份失败：{}", err));
                            }
                        }
                        Loading::hide();
                        pending.store(false, Ordering::Relaxed);
                    });
                } else {
                    Toast::show("备份取消！".to_string());
                }
            }
            None => {
                Toast::show("没有找到游戏存档！".to_string());
            }
        }
    }

    fn do_delete_game_save(&self, backup_name: &str) {
        let backup_name = format!("{}/{}", self.local_dir, backup_name);
        let local_dir = self.local_dir();
        let items = Arc::clone(&self.items);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            match fs::remove_file(&backup_name) {
                Ok(_) => {
                    get_local_game_saves(local_dir, items);
                    Toast::show("删除完成！".to_string());
                }
                Err(err) => {
                    error!("delete {} failed: {:?}", backup_name, err);
                    Toast::show(format!("删除失败：{}！", err));
                }
            }
            Loading::hide();
            pending.store(false, Ordering::Relaxed);
        });
    }

    fn update(&mut self, game_save_dir: &Option<String>, buttons: u32) {
        self.scroll_progress.update(buttons);
        // do backup
        let selected_idx = self.list_state.selected_idx;
        let idx = selected_idx - 1;
        if is_button(buttons, SceCtrlButtons::SceCtrlCircle) {
            if selected_idx == 0 {
                // 新建备份
                self.do_backup_game_save(game_save_dir, None);
            } else {
                // 覆盖备份
                let back_name = self
                    .get_items()
                    .get((selected_idx - 1) as usize)
                    .unwrap()
                    .to_string();
                if UIDialog::present(&format!("覆盖当前备份：{}？", back_name)) {
                    self.do_backup_game_save(game_save_dir, Some(back_name));
                }
            }
        } else if idx >= 0 {
            if is_button(buttons, SceCtrlButtons::SceCtrlSquare) {
                let backup_name = &self.get_items().get(idx as usize).unwrap().to_owned();
                if UIDialog::present(&format!("使用备份还原游戏：{}？", backup_name)) {
                    self.do_restore_game_save(game_save_dir, backup_name);
                }
            } else if is_button(buttons, SceCtrlButtons::SceCtrlTriangle) {
                let backup_name = &self.get_items().get(idx as usize).unwrap().to_owned();
                if UIDialog::present(&format!("删除备份：{}？", backup_name)) {
                    self.do_delete_game_save(backup_name);
                }
            } else if is_button(buttons, SceCtrlButtons::SceCtrlSelect) {
                if Api::is_eat_pancake_valid() {
                    self.upload_backup();
                } else {
                    UIDialog::present_qrcode(HOME_PAGE_URL);
                }
            }
        }

        // update list state
        let size = (self.get_items().len() + 1) as i32;
        self.list_state.update(size, buttons);
    }

    fn draw(&self, left: i32, top: i32) {
        let items = self.get_items();
        let size = items.len() as i32;
        let ListState {
            top_row,
            selected_idx,
            display_row,
        } = self.list_state;
        for idx in 0..display_row {
            let i = top_row + idx;
            if i > size {
                break;
            }
            let mut x = left + 12;
            let y = top + 68;
            let h = 30 * idx;
            if i == selected_idx {
                vita2d_draw_rect(
                    x as f32,
                    (y + h - 21) as f32,
                    (SCREEN_WIDTH / 2 - 24) as f32,
                    30.0,
                    get_active_color(),
                );
                vita2d_draw_rect(
                    (x + 2) as f32,
                    (y + 2 + h - 21) as f32,
                    (SCREEN_WIDTH / 2 - 28) as f32,
                    26.0,
                    rgba(0x18, 0x18, 0x18, 0xff),
                );
            }

            if i == 0 {
                vita2d_draw_text(
                    x + 8,
                    y + h,
                    rgba(0xff, 0xff, 0xff, 0xff),
                    1.0,
                    self.new_backup_text,
                );
            } else if let Some(name) = items.get((i - 1) as usize) {
                x = x + 8;
                let text_width = vita2d_text_width(1.0, name);
                if text_width > LIST_NAME_WIDTH {
                    vita2d_set_clip(
                        x,
                        y + 2 + h - 21,
                        x + LIST_NAME_WIDTH,
                        (y + 2 + h - 21) + 26,
                    );
                    if i == selected_idx {
                        x = x
                            - ((text_width - LIST_NAME_WIDTH) as f32
                                * self.scroll_progress.progress())
                                as i32;
                    }
                }
                vita2d_draw_text(x, y + h, rgba(0xff, 0xff, 0xff, 0xff), 1.0, name);
                if text_width > LIST_NAME_WIDTH {
                    vita2d_unset_clip();
                }
            }
        }
    }
}
