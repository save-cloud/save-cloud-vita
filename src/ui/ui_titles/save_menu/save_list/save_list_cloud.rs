use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock, RwLockReadGuard,
    },
};

use log::{error, info};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::{
    api::{Api, AuthData},
    constant::{
        GAME_SAVE_CLOUD_DIR, HOME_PAGE_URL, LIST_NAME_WIDTH, SAVE_LIST_QR_CODE_SIZE,
        SCAN_QR_CODE_TIPS, SCREEN_WIDTH,
    },
    ime::{get_current_format_time, show_keyboard},
    tai::{mount_pfs, Title},
    ui::{
        ui_cloud::list_state::ListState,
        ui_dialog::UIDialog,
        ui_list::UIList,
        ui_loading::{draw_loading, Loading},
        ui_scroll_progress::ScrollProgress,
        ui_toast::Toast,
    },
    utils::{
        backup_game_save, delete_dir_if_empty, get_game_local_backup_dir, normalize_path,
        restore_game_save,
    },
    vita2d::{
        is_button, rgba, vita2d_draw_rect, vita2d_draw_text, vita2d_draw_texture,
        vita2d_load_png_buf, vita2d_set_clip, vita2d_text_width, vita2d_unset_clip, SceCtrlButtons,
        Vita2dTexture,
    },
};
use crate::{constant::SCREEN_HEIGHT, utils::get_active_color};

use super::DISPLAY_ROW;

pub struct SaveItem {
    pub name: String,
    pub fs_id: u64,
}

pub struct QrCodeState {
    pub qr_code: Option<Vita2dTexture>,
    pub qr_code_buf: Arc<RwLock<Option<Vec<u8>>>>,
}

impl QrCodeState {
    pub fn new() -> QrCodeState {
        QrCodeState {
            qr_code: None,
            qr_code_buf: Arc::new(RwLock::new(None)),
        }
    }
}

pub struct SaveListCloud {
    pending: Arc<AtomicBool>,
    list_state: ListState,
    local_dir: String,
    cloud_dir: Arc<RwLock<String>>,
    title_id: String,
    items: Arc<RwLock<Option<Vec<SaveItem>>>>,
    qr_code_state: QrCodeState,
    new_backup_text: &'static str,
    scroll_progress: ScrollProgress,
}

impl SaveListCloud {
    pub fn new(new_back: &'static str, title: &Title) -> SaveListCloud {
        SaveListCloud {
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
            items: Arc::new(RwLock::new(None)),
            qr_code_state: QrCodeState::new(),
            new_backup_text: new_back,
            scroll_progress: ScrollProgress::new(40.0, 100.0),
        }
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

    fn get_items(&self) -> RwLockReadGuard<Option<Vec<SaveItem>>> {
        self.items.read().expect("read game saves")
    }

    fn is_list_ready(&self) -> bool {
        self.get_items().is_some()
    }

    pub fn get_size(&self) -> usize {
        match *self.get_items() {
            Some(ref items) => items.len(),
            None => 0,
        }
    }

    pub fn get_item_by_idx(&self, idx: usize) -> Option<(String, u64)> {
        match *self.get_items() {
            Some(ref items) => {
                let item = items.get(idx);
                if item.is_none() {
                    None
                } else {
                    let item = item.unwrap();
                    Some((item.name.to_string(), item.fs_id))
                }
            }
            None => None,
        }
    }

    pub fn get_item_name_by_idx(&self, idx: usize) -> Option<String> {
        match *self.get_items() {
            Some(ref items) => {
                let item = items.get(idx);
                if item.is_none() {
                    None
                } else {
                    Some(item.unwrap().name.to_string())
                }
            }
            None => None,
        }
    }

    pub fn get_item_fs_id_by_idx(&self, idx: usize) -> Option<u64> {
        match *self.get_items() {
            Some(ref items) => {
                let item = items.get(idx);
                if item.is_none() {
                    None
                } else {
                    Some(item.unwrap().fs_id)
                }
            }
            None => None,
        }
    }

    fn draw_list(&self, left: i32, top: i32) {
        let size = self.get_size() as i32;
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
            } else if let Some(name) = self.get_item_name_by_idx((i - 1) as usize) {
                x = x + 8;
                let text_width = vita2d_text_width(1.0, &name);
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
                vita2d_draw_text(x, y + h, rgba(0xff, 0xff, 0xff, 0xff), 1.0, &name);
                if text_width > LIST_NAME_WIDTH {
                    vita2d_unset_clip();
                }
            }
        }
    }

    fn is_fetch_cloud_save_list(&self) -> bool {
        Arc::strong_count(&self.items) > 1
    }

    fn fetch_save_list(&mut self) {
        if self.is_fetch_cloud_save_list() {
            return;
        }
        let dir = Arc::clone(&self.cloud_dir);
        let items = Arc::clone(&self.items);
        let title_id = self.title_id.to_string();
        tokio::spawn(async move {
            let (game_save_dir, res) = Api::fetch_save_cloud_list(&title_id, false);
            if let Some(game_save_dir) = game_save_dir {
                *dir.write().expect("write save dir") = game_save_dir;
            }
            *items.write().expect("write game saves") = res;
        });
    }

    fn start_auth(&mut self) {
        let qr_code_buf = Arc::clone(&self.qr_code_state.qr_code_buf);
        let dir = Arc::clone(&self.cloud_dir);
        let items = Arc::clone(&self.items);
        let title_id = self.title_id.to_string();
        tokio::spawn(async move {
            let api_type = Api::get_read().api_type;
            let auth_url = Api::get_read().get_auth_url();
            let device_code = match Api::start_auth(&auth_url, api_type) {
                Ok(auth_res) => {
                    let qrcode_url = Api::get_read()
                        .get_qr_code_url(&auth_res.user_code.expect("auth user code"));
                    let buf = qrcode_generator::to_png_to_vec(
                        qrcode_url,
                        qrcode_generator::QrCodeEcc::Low,
                        SAVE_LIST_QR_CODE_SIZE as usize,
                    )
                    .unwrap();
                    *qr_code_buf.write().unwrap() = Some(buf);
                    auth_res.device_code
                }
                Err(err) => {
                    error!("auth error: {:?}", err);
                    Toast::show(format!("获取授权失败"));
                    None
                }
            };

            while let Some(device_code) = &device_code {
                let get_token_url = Api::get_read().get_token_url(device_code);
                match Api::start_fetch_token(&get_token_url, api_type) {
                    Ok(token_res) => {
                        match Api::start_fetch_name_of_pancake(
                            token_res.access_token.as_ref().unwrap(),
                        ) {
                            Ok(name_of_pancake) => {
                                // 更新登录状态
                                Api::update_auth_data(
                                    api_type,
                                    Some(AuthData::new(token_res, name_of_pancake)),
                                );
                                // 获取云端存档列表
                                let (game_save_dir, res) =
                                    Api::fetch_save_cloud_list(&title_id, false);
                                if let Some(game_save_dir) = game_save_dir {
                                    *dir.write().expect("write save dir") = game_save_dir;
                                }
                                *items.write().expect("write game saves") = res;
                                Toast::show("登录成功！".to_string());
                            }
                            Err(err) => {
                                error!("fetch profile failed: {:?}", err);
                                Toast::show("登录失败，获取用户信息失败！".to_string());
                            }
                        }
                        break;
                    }
                    Err(err) => {
                        info!("fetch token failed: {:?}", err);
                    }
                }

                // wait 6s
                tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

                // drop if strong count < 2
                if Arc::strong_count(&qr_code_buf) < 2 {
                    break;
                }
            }
        });
    }

    fn download_cloud_backup(&self, game_save_dir: &Option<String>, restore: bool) {
        let idx = self.list_state.selected_idx - 1;
        if idx < 0 {
            return;
        }
        if let Some(game_save_dir) = game_save_dir {
            let game_save_dir = game_save_dir.to_string();
            let (backup_name, fs_id) = self.get_item_by_idx(idx as usize).unwrap();
            let download_backup_name = if !restore {
                backup_name.to_string()
            } else {
                format!("{}.zip", &get_current_format_time() as &str)
            };
            let local_dir = self.local_dir();
            let download_to_path = format!("{}/{}", local_dir, download_backup_name);
            if !Path::new(&download_to_path).exists() {
                if UIDialog::present(&if restore {
                    format!("使用云备份还原游戏：{}？", backup_name)
                } else {
                    format!("下载云备份：{}？", backup_name)
                }) {
                    let pending = Arc::clone(&self.pending);
                    pending.store(true, Ordering::Relaxed);
                    Loading::show();
                    if restore {
                        mount_pfs(&game_save_dir);
                    }
                    tokio::spawn(async move {
                        Loading::notify_title("正在下载云备份".to_string());
                        Loading::notify_desc(backup_name);
                        let is_success = match Api::start_download(fs_id, &download_to_path) {
                            Ok(_) => true,
                            Err(err) => {
                                error!(
                                    "download {} from cloud failed: {:?}",
                                    download_to_path, err
                                );
                                Toast::show(format!("云备份下载失败"));
                                false
                            }
                        };
                        if is_success {
                            if restore {
                                Loading::notify_title("正在恢复存档".to_string());
                                match restore_game_save(&download_to_path, &game_save_dir) {
                                    Ok(_) => {
                                        Toast::show("存档恢复完成！".to_string());
                                    }
                                    Err(err) => {
                                        error!(
                                            "extract zip {} to {} failed: {:?}",
                                            download_to_path, game_save_dir, err
                                        );
                                        Toast::show(format!("存档恢复失败：{}", err));
                                    }
                                }
                                // remove local backup after restore
                                if Path::new(&download_to_path).exists() {
                                    if let Err(err) = fs::remove_file(&download_to_path) {
                                        error!(
                                            "remove {} failed after backup restore: {:?}",
                                            download_to_path, err
                                        );
                                    }
                                    let _ = delete_dir_if_empty(&local_dir);
                                }
                            } else {
                                // update save list
                                Toast::show("云备份下载完成！".to_string());
                            }
                        }
                        Loading::hide();
                        pending.store(false, Ordering::Relaxed);
                    });
                }
            } else {
                Toast::show("本地已存在同名备份！".to_string());
            }
        }
    }
}

impl UIList for SaveListCloud {
    fn init(&mut self) {
        if Arc::strong_count(&self.qr_code_state.qr_code_buf) > 1 {
            return;
        }
        if Api::get_read().is_login() {
            self.fetch_save_list();
        } else {
            self.start_auth();
        }
    }

    fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }

    fn do_backup_game_save(&self, game_save_dir: &Option<String>, input_overwrite: Option<String>) {
        match &game_save_dir {
            Some(game_save_dir) => {
                let game_save_dir = game_save_dir.to_string();
                let local_dir = self.local_dir();
                let input = match &input_overwrite {
                    Some(input) => format!("{}", input),
                    None => {
                        let input = show_keyboard(&get_current_format_time());
                        if input.len() > 0 {
                            format!("{}.zip", input)
                        } else {
                            "".to_string()
                        }
                    }
                };
                if input.len() > 0 {
                    let backup_name = format!("{}/{}", local_dir, input);
                    let is_overwrite = input_overwrite.is_some();
                    let cloud_dir = self.cloud_dir();
                    let dir = Arc::clone(&self.cloud_dir);
                    let items = Arc::clone(&self.items);
                    let title_id = self.title_id.to_string();
                    let pending = Arc::clone(&self.pending);
                    pending.store(true, Ordering::Relaxed);
                    Loading::show();
                    mount_pfs(&game_save_dir);
                    tokio::spawn(async move {
                        Loading::notify_title("正在云备份".to_string());
                        match backup_game_save(&game_save_dir, &backup_name) {
                            Ok(_) => {
                                match Api::upload_to_cloud(
                                    &cloud_dir,
                                    &input,
                                    &backup_name,
                                    is_overwrite,
                                ) {
                                    Ok(_) => {
                                        // 获取云端存档列表
                                        let (game_save_dir, res) =
                                            Api::fetch_save_cloud_list(&title_id, false);
                                        if let Some(game_save_dir) = game_save_dir {
                                            *dir.write().expect("write save dir") = game_save_dir;
                                        }
                                        *items.write().expect("write game saves") = res;
                                        // update save list
                                        Toast::show(if !is_overwrite {
                                            "新建云备份完成！".to_string()
                                        } else {
                                            "云备份覆盖完成！".to_string()
                                        });
                                    }
                                    Err(err) => {
                                        error!("upload {} to cloud failed: {:?}", backup_name, err);
                                        Toast::show(format!("云备份上传失败"));
                                    }
                                }
                            }
                            Err(err) => {
                                error!(
                                    "zip {} to {} failed: {:?}",
                                    game_save_dir, backup_name, err
                                );
                                Toast::show(format!("云备份失败"));
                            }
                        }
                        // remove local backup after upload
                        if Path::new(&backup_name).exists() {
                            if let Err(err) = fs::remove_file(&backup_name) {
                                error!(
                                    "remove {} failed after backup upload: {}",
                                    backup_name, err
                                );
                            }
                            let _ = delete_dir_if_empty(&local_dir);
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
        let backup_name = format!("{}/{}", self.cloud_dir(), backup_name);
        let title_id = self.title_id.to_string();
        let dir = Arc::clone(&self.cloud_dir);
        let items = Arc::clone(&self.items);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            Loading::notify_title("正在删除云备份".to_string());
            Loading::notify_desc(backup_name.split("/").last().unwrap_or("").to_string());
            match Api::start_file_manager(
                &utf8_percent_encode(&backup_name, NON_ALPHANUMERIC).to_string(),
                None,
                None,
                crate::api::ApiOperates::Delete,
            ) {
                Ok(_) => {
                    let (game_save_dir, res) = Api::fetch_save_cloud_list(&title_id, false);
                    if let Some(game_save_dir) = game_save_dir {
                        *dir.write().expect("write save dir") = game_save_dir;
                    }
                    *items.write().expect("write game saves") = res;
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
                if self.is_list_ready() {
                    if Api::is_eat_pancake_valid() {
                        // 新建备份
                        self.do_backup_game_save(game_save_dir, None);
                    } else {
                        UIDialog::present_qrcode(HOME_PAGE_URL);
                    }
                }
            } else {
                if Api::is_eat_pancake_valid() {
                    // 覆盖备份
                    let back_name = self
                        .get_item_name_by_idx((selected_idx - 1) as usize)
                        .unwrap()
                        .to_string();
                    if UIDialog::present(&format!("覆盖当前备份：{}？", back_name)) {
                        self.do_backup_game_save(game_save_dir, Some(back_name));
                    }
                } else {
                    UIDialog::present_qrcode(HOME_PAGE_URL);
                }
            }
        } else if idx >= 0 {
            if is_button(buttons, SceCtrlButtons::SceCtrlTriangle) {
                if Api::is_eat_pancake_valid() {
                    let backup_name = &self.get_item_name_by_idx(idx as usize).unwrap();
                    if UIDialog::present(&format!("删除云备份：{}？", backup_name)) {
                        self.do_delete_game_save(backup_name);
                    }
                } else {
                    UIDialog::present_qrcode(HOME_PAGE_URL);
                }
            } else if is_button(buttons, SceCtrlButtons::SceCtrlSelect) {
                if Api::is_eat_pancake_valid() {
                    self.download_cloud_backup(game_save_dir, false);
                } else {
                    UIDialog::present_qrcode(HOME_PAGE_URL);
                }
            } else if is_button(buttons, SceCtrlButtons::SceCtrlSquare) {
                if Api::is_eat_pancake_valid() {
                    self.download_cloud_backup(game_save_dir, true);
                } else {
                    UIDialog::present_qrcode(HOME_PAGE_URL);
                }
            }
        }

        // update scroll
        self.list_state
            .update((self.get_size() + 1) as i32, buttons);

        // update qrcode Texture
        if let Ok(mut qr_code) = self.qr_code_state.qr_code_buf.try_write() {
            if let Some(buf) = &*qr_code {
                self.qr_code_state.qr_code = Some(vita2d_load_png_buf(buf));
                *qr_code = None;
            }
        } else if self.qr_code_state.qr_code.is_some() && Api::get_read().is_login() {
            self.qr_code_state.qr_code = None;
        }
    }

    fn draw(&self, left: i32, top: i32) {
        self.draw_list(left, top);
        if self.qr_code_state.qr_code.is_some() {
            let x = (left + (SCREEN_WIDTH / 2 - SAVE_LIST_QR_CODE_SIZE) / 2) as f32;
            let y = (top + 90) as f32;
            vita2d_draw_texture(self.qr_code_state.qr_code.as_ref().unwrap(), x, y);
            vita2d_draw_text(
                left + (SCREEN_WIDTH / 2 - vita2d_text_width(1.0, SCAN_QR_CODE_TIPS)) / 2,
                SCREEN_HEIGHT - 100,
                rgba(0xff, 0xff, 0xff, 0xff),
                1.0,
                SCAN_QR_CODE_TIPS,
            )
        } else if !self.is_list_ready() {
            draw_loading((left + 12) as f32, (SCREEN_HEIGHT - 104) as f32, 15.0);
        }
    }
}
