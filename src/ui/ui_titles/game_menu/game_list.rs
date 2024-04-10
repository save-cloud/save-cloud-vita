use std::{
    fmt::{Display, Formatter},
    fs,
    ops::Deref,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};

use log::error;

use crate::{
    api::Api,
    constant::{
        GAME_CARD_SAVE_DIR, GAME_SAVE_CLOUD_DIR, GAME_SAVE_DIR, HOME_PAGE_URL, SCREEN_WIDTH,
    },
    ime::get_current_format_time,
    tai::{mount_pfs, psv_launch_app_by_title_id, unmount_pfs, Title, Titles},
    ui::{
        ui_cloud::list_state::ListState, ui_dialog::UIDialog, ui_loading::Loading, ui_toast::Toast,
    },
    utils::{
        backup_game_save, delete_dir_if_empty, get_active_color, get_game_local_backup_dir,
        normalize_path, update_sfo_file_with_current_account_id,
    },
    vita2d::{is_button, rgba, vita2d_draw_rect, vita2d_draw_text, SceCtrlButtons},
};

enum GameMenuAction {
    BackupAllGameSave,
    BackupAllGameSaveToCloud,
    ChangeAccountId,
    DeleteGameSave,
    DeleteSelectedGameSave,
    DeleteAllGameSaves,
    LaunchApp,
}

impl Deref for GameMenuAction {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            GameMenuAction::BackupAllGameSave => "备份所有游戏存档",
            GameMenuAction::BackupAllGameSaveToCloud => "备份所有游戏存档到云端",
            GameMenuAction::ChangeAccountId => "修改存档账号为当前账号",
            GameMenuAction::DeleteGameSave => "删除该游戏存档",
            GameMenuAction::DeleteSelectedGameSave => "删除该游戏本地存档备份",
            GameMenuAction::DeleteAllGameSaves => "删除所有游戏本地存档备份",
            GameMenuAction::LaunchApp => "启动游戏",
        }
    }
}

impl Display for GameMenuAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.deref())
    }
}

pub struct GameList {
    pending: Arc<AtomicBool>,
    list_state: ListState,
    list: [GameMenuAction; 7],
    game_save_dir_prepare_to_mount: Arc<RwLock<Option<String>>>,
    game_save_dir_on_mounted: Arc<RwLock<Option<String>>>,
}

impl GameList {
    pub fn new() -> Self {
        GameList {
            pending: Arc::new(AtomicBool::new(false)),
            list_state: ListState::new(15),
            list: [
                GameMenuAction::LaunchApp,
                GameMenuAction::BackupAllGameSave,
                GameMenuAction::BackupAllGameSaveToCloud,
                GameMenuAction::ChangeAccountId,
                GameMenuAction::DeleteGameSave,
                GameMenuAction::DeleteSelectedGameSave,
                GameMenuAction::DeleteAllGameSaves,
            ],
            game_save_dir_prepare_to_mount: Arc::new(RwLock::new(None)),
            game_save_dir_on_mounted: Arc::new(RwLock::new(None)),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.pending.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn delete_game_save(&self, title: &Title) {
        let real_id = title.real_id().to_string();
        let name = title.name().to_string();
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        unmount_pfs();
        tokio::spawn(async move {
            let dirs = [
                format!("{}/{}", GAME_CARD_SAVE_DIR, real_id),
                format!("{}/{}", GAME_SAVE_DIR, real_id),
            ];
            if let Some(game_save_dir) = dirs.iter().find(|dir| Path::new(&dir).exists()) {
                if let Err(err) = fs::remove_dir_all(&game_save_dir) {
                    error!("remove {} failed: {}", game_save_dir, err);
                    Toast::show(format!("删除 {} 存档失败！", name));
                } else {
                    Toast::show(format!("删除 {} 存档完成！", name));
                }
            } else {
                Toast::show(format!("{} 存档不存在！", name));
            }
            Loading::hide();
            pending.store(false, Ordering::Relaxed);
        });
    }

    pub fn delete_selected_game_save(&self, title: &Title) {
        let title_id = title.title_id().to_string();
        let name = title.name().to_string();
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            let local_dir = get_game_local_backup_dir(&title_id, &name);
            if Path::new(&local_dir).exists() {
                if let Err(err) = fs::remove_dir_all(&local_dir) {
                    error!("remove {} failed: {}", local_dir, err);
                    Toast::show(format!("删除 {} 本地备份失败！", name));
                } else {
                    Toast::show(format!("删除 {} 游戏本地备份完成！", name));
                }
            } else {
                Toast::show(format!("{} 本地备份不存在！", name));
            }
            Loading::hide();
            pending.store(false, Ordering::Relaxed);
        });
    }

    pub fn delete_all_game_saves(&self, titles: &Titles) {
        let list = titles
            .iter()
            .map(|title| (title.title_id().to_string(), title.name().to_string()))
            .collect::<Vec<(String, String)>>();

        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            let mut delete_failed_count = 0;
            for (_idx, (title_id, name)) in list.iter().enumerate() {
                let local_dir = get_game_local_backup_dir(&title_id, &name);
                if Path::new(&local_dir).exists() {
                    if let Err(err) = fs::remove_dir_all(&local_dir) {
                        error!("remove {} failed: {}", local_dir, err);
                        Toast::show(format!("删除 {} 本地备份失败！", name));
                        delete_failed_count += 1;
                    }
                }
            }
            if delete_failed_count == 0 {
                Toast::show("删除所有游戏备份完成！".to_string());
            } else {
                Toast::show(format!(
                    "删除部分游戏备份完成，{} 个删除失败",
                    delete_failed_count
                ));
            }
            Loading::hide();
            pending.store(false, Ordering::Relaxed);
        });
    }

    pub fn backup_all_game_save(&self, titles: &Titles) {
        let list = titles
            .iter()
            .map(|title| {
                (
                    title.title_id().to_string(),
                    title.real_id().to_string(),
                    title.name().to_string(),
                )
            })
            .collect::<Vec<(String, String, String)>>();

        let game_save_dir_on_mounted = Arc::clone(&self.game_save_dir_on_mounted);
        let game_save_dir_prepare_to_mount = Arc::clone(&self.game_save_dir_prepare_to_mount);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            let mut backup_failed_count = 0;
            for (idx, (title_id, real_id, name)) in list.iter().enumerate() {
                Loading::notify_title(format!(
                    "正在备份 ({}/{})： {}！",
                    idx + 1,
                    list.len(),
                    name
                ));
                let dirs = [
                    format!("{}/{}", GAME_CARD_SAVE_DIR, real_id),
                    format!("{}/{}", GAME_SAVE_DIR, real_id),
                ];
                let game_save_dir = dirs.iter().find(|dir| Path::new(&dir).exists());
                if game_save_dir.is_none() {
                    continue;
                }
                let game_save_dir = game_save_dir.unwrap();
                let mut is_prepare = false;
                loop {
                    if let Ok(game_save_dir_on_mounted) = game_save_dir_on_mounted.try_read() {
                        if let Some(game_save_dir_on_mounted) = game_save_dir_on_mounted.as_ref() {
                            if game_save_dir_on_mounted == game_save_dir {
                                break;
                            }
                        }
                    }
                    if !is_prepare {
                        if let Ok(mut game_save_dir_prepare_to_mount) =
                            game_save_dir_prepare_to_mount.try_write()
                        {
                            is_prepare = true;
                            *game_save_dir_prepare_to_mount = Some(game_save_dir.clone());
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                let backup_to_path = format!(
                    "{}/{}.zip",
                    get_game_local_backup_dir(&title_id, &name),
                    get_current_format_time()
                );
                match backup_game_save(game_save_dir, &backup_to_path) {
                    Err(err) => {
                        backup_failed_count += 1;
                        error!(
                            "zip {} to {} failed: {:?}",
                            game_save_dir, backup_to_path, err
                        );
                        Toast::show(format!("游戏 {} 备份失败！", name));
                    }
                    _ => {}
                }
            }
            if backup_failed_count == 0 {
                Toast::show("所有游戏备份完成！".to_string());
            } else {
                Toast::show(format!(
                    "部分游戏备份完成，{} 个备份失败",
                    backup_failed_count
                ));
            }
            Loading::hide();
            pending.store(false, Ordering::Relaxed);
        });
    }

    pub fn backup_all_game_save_to_cloud(&self, titles: &Titles) {
        let list = titles
            .iter()
            .map(|title| {
                (
                    title.title_id().to_string(),
                    title.real_id().to_string(),
                    title.name().to_string(),
                )
            })
            .collect::<Vec<(String, String, String)>>();

        let game_save_dir_on_mounted = Arc::clone(&self.game_save_dir_on_mounted);
        let game_save_dir_prepare_to_mount = Arc::clone(&self.game_save_dir_prepare_to_mount);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            let mut backup_failed_count = 0;
            for (idx, (title_id, real_id, name)) in list.iter().enumerate() {
                Loading::notify_title(format!(
                    "正在备份 ({}/{})： {}！",
                    idx + 1,
                    list.len(),
                    name
                ));
                let dirs = [
                    format!("{}/{}", GAME_CARD_SAVE_DIR, real_id),
                    format!("{}/{}", GAME_SAVE_DIR, real_id),
                ];
                let game_save_dir = dirs.iter().find(|dir| Path::new(&dir).exists());
                if game_save_dir.is_none() {
                    continue;
                }
                let game_save_dir = game_save_dir.unwrap();
                let mut is_prepare = false;
                loop {
                    if let Ok(game_save_dir_on_mounted) = game_save_dir_on_mounted.try_read() {
                        if let Some(game_save_dir_on_mounted) = game_save_dir_on_mounted.as_ref() {
                            if game_save_dir_on_mounted == game_save_dir {
                                break;
                            }
                        }
                    }
                    if !is_prepare {
                        if let Ok(mut game_save_dir_prepare_to_mount) =
                            game_save_dir_prepare_to_mount.try_write()
                        {
                            is_prepare = true;
                            *game_save_dir_prepare_to_mount = Some(game_save_dir.clone());
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                let backup_name = format!("{}.zip", get_current_format_time());
                let local_dir = get_game_local_backup_dir(&title_id, &name);
                let backup_to_path = format!("{}/{}", local_dir, backup_name);
                let success = match backup_game_save(game_save_dir, &backup_to_path) {
                    Err(err) => {
                        backup_failed_count += 1;
                        error!(
                            "zip {} to {} failed: {:?}",
                            game_save_dir, backup_to_path, err
                        );
                        Toast::show(format!("游戏 {} 备份失败！", name));
                        false
                    }
                    _ => true,
                };

                if success {
                    let (cloud_dir, _) = Api::fetch_save_cloud_list(&title_id, true);
                    let cloud_dir = if cloud_dir.is_some() {
                        cloud_dir.unwrap()
                    } else {
                        format!(
                            "{}/{} {}",
                            GAME_SAVE_CLOUD_DIR,
                            title_id,
                            normalize_path(name.trim())
                        )
                        .trim()
                        .to_string()
                    };
                    match Api::upload_to_cloud(&cloud_dir, &backup_name, &backup_to_path, false) {
                        Err(err) => {
                            error!("upload {} to cloud failed: {:?}", backup_to_path, err);
                            Toast::show(format!("游戏 {} 备份上传失败！", title_id));
                        }
                        _ => {}
                    }
                }

                // remove local backup after upload
                if Path::new(&backup_to_path).exists() {
                    if let Err(err) = fs::remove_file(&backup_to_path) {
                        error!(
                            "remove {} failed after backup upload: {}",
                            backup_to_path, err
                        );
                    }
                    let _ = delete_dir_if_empty(&local_dir);
                }
            }
            if backup_failed_count == 0 {
                Toast::show("所有游戏备份完成！".to_string());
            } else {
                Toast::show(format!(
                    "部分游戏备份完成，{} 个备份失败！",
                    backup_failed_count
                ));
            }
            Loading::hide();
            pending.store(false, Ordering::Relaxed);
        });
    }

    pub fn mount_game_dir_if_exists(&self) {
        let prepare_dir = match self.game_save_dir_prepare_to_mount.try_write() {
            Ok(mut prepare_dir) => {
                if prepare_dir.is_none() {
                    None
                } else {
                    Some(prepare_dir.take().unwrap())
                }
            }
            _ => None,
        };

        // mount
        if let Some(prepare_dir) = prepare_dir {
            mount_pfs(&prepare_dir);
            *self.game_save_dir_on_mounted.write().unwrap() = Some(prepare_dir);
        }
    }

    pub fn update(&mut self, buttons: u32, title: &Title, titles: &Titles) {
        self.mount_game_dir_if_exists();

        if self.is_pending() {
            return;
        }
        let ListState { selected_idx, .. } = self.list_state;
        if is_button(buttons, SceCtrlButtons::SceCtrlCircle) {
            let action = &self.list[selected_idx as usize];
            match action {
                GameMenuAction::LaunchApp => {
                    if UIDialog::present(&format!(
                        "{}: {}",
                        &GameMenuAction::LaunchApp,
                        title.name()
                    )) {
                        psv_launch_app_by_title_id(title.title_id());
                    }
                }
                GameMenuAction::BackupAllGameSave => {
                    if UIDialog::present(&GameMenuAction::BackupAllGameSave) {
                        self.backup_all_game_save(titles);
                    }
                }
                GameMenuAction::BackupAllGameSaveToCloud => {
                    if Api::get_read().is_login() {
                        if Api::is_eat_pancake_valid() {
                            if UIDialog::present(&GameMenuAction::BackupAllGameSaveToCloud) {
                                self.backup_all_game_save_to_cloud(titles);
                            }
                        } else {
                            UIDialog::present_qrcode(HOME_PAGE_URL);
                        }
                    } else {
                        Toast::show("请先登录！".to_string());
                    }
                }
                GameMenuAction::ChangeAccountId => {
                    if UIDialog::present(&GameMenuAction::ChangeAccountId) {
                        [
                            format!("{}/{}", GAME_CARD_SAVE_DIR, title.real_id()),
                            format!("{}/{}", GAME_SAVE_DIR, title.real_id()),
                        ]
                        .iter()
                        .any(|path| {
                            let sfo_path = format!("{}/sce_sys/param.sfo", path);
                            if Path::new(&sfo_path).exists() {
                                mount_pfs(path);
                                if let Ok(()) = update_sfo_file_with_current_account_id(&sfo_path) {
                                    Toast::show("修改存档为当前账号完成！".to_string());
                                } else {
                                    Toast::show("修改存档为当前账号失败！".to_string());
                                }
                                unmount_pfs();
                                return true;
                            }
                            false
                        });
                    }
                }
                GameMenuAction::DeleteGameSave => {
                    let mut count = 3;
                    loop {
                        if UIDialog::present(&if count == 0 {
                            format!("{}", GameMenuAction::DeleteGameSave)
                        } else {
                            format!("{}：{}", GameMenuAction::DeleteGameSave, count)
                        }) {
                            if count == 0 {
                                self.delete_game_save(title);
                                break;
                            } else {
                                count -= 1;
                            }
                        } else {
                            break;
                        }
                    }
                }
                GameMenuAction::DeleteSelectedGameSave => {
                    let mut count = 3;
                    loop {
                        if UIDialog::present(&if count == 0 {
                            format!("{}", GameMenuAction::DeleteSelectedGameSave)
                        } else {
                            format!("{}：{}", GameMenuAction::DeleteSelectedGameSave, count)
                        }) {
                            if count == 0 {
                                self.delete_selected_game_save(title);
                                break;
                            } else {
                                count -= 1;
                            }
                        } else {
                            break;
                        }
                    }
                }
                GameMenuAction::DeleteAllGameSaves => {
                    let mut count = 3;
                    loop {
                        if UIDialog::present(&if count == 0 {
                            format!("{}", GameMenuAction::DeleteAllGameSaves)
                        } else {
                            format!("{}：{}", GameMenuAction::DeleteAllGameSaves, count)
                        }) {
                            if count == 0 {
                                self.delete_all_game_saves(titles);
                                break;
                            } else {
                                count -= 1;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        self.list_state.update(self.list.len() as i32, buttons);
    }

    pub fn draw(&self, left: i32, top: i32) {
        let actions = &self.list;
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

            vita2d_draw_text(
                x + 8,
                y + 30 * idx,
                rgba(0xff, 0xff, 0xff, 0xff),
                1.0,
                &actions[i as usize],
            );
        }
    }
}
