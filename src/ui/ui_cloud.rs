use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};

use log::{error, info};

use crate::{
    api::{Api, AuthData},
    app::AppData,
    constant::{
        HOME_PAGE_URL, SAVE_LIST_QR_CODE_SIZE, SCAN_QR_CODE_TIPS, SCREEN_HEIGHT, SCREEN_WIDTH,
    },
    ime::{get_current_format_time, show_keyboard},
    tai::{mount_pfs, unmount_pfs},
    ui::ui_toast::Toast,
    utils::{
        copy_dir_all, join_path, normalize_path, update_sfo_file_with_current_account_id, zip_dir,
        zip_extract, zip_file,
    },
    vita2d::{
        is_button, rgba, vita2d_draw_text, vita2d_draw_texture, vita2d_line, vita2d_load_png_buf,
        vita2d_load_png_file, vita2d_set_clip, vita2d_text_height, vita2d_text_width,
        vita2d_unset_clip, SceCtrlButtons, Vita2dTexture,
    },
};

use self::{
    action::{do_cloud_action, do_local_action},
    menu::Menu,
    panel::{DirPending, DirPendingAction, Panel},
};

use super::{
    ui_base::UIBase, ui_dialog::UIDialog, ui_loading::Loading, ui_scroll_progress::ScrollProgress,
    ui_titles::save_menu::save_list::save_list_cloud::QrCodeState,
};

pub mod action;
pub mod list_state;
pub mod menu;
pub mod panel;

pub struct UICloud {
    pub pending: Arc<AtomicBool>,
    pub active_panel: usize,
    pub right_panel: usize,
    pub panels: [Panel; 3],
    pub no_data_tex: Option<Vita2dTexture>,
    pub qr_code_state: QrCodeState,
    pub menu: Menu,
    pub scroll_progress: ScrollProgress,
}

impl UICloud {
    pub fn new() -> UICloud {
        UICloud {
            pending: Arc::new(AtomicBool::new(false)),
            active_panel: 0,
            right_panel: 2,
            panels: [
                panel::Panel::new_local(12),
                panel::Panel::new_local(SCREEN_WIDTH / 2 + 12),
                panel::Panel::new_cloud(SCREEN_WIDTH / 2 + 12),
            ],
            no_data_tex: None,
            qr_code_state: QrCodeState::new(),
            menu: Menu::new(),
            scroll_progress: ScrollProgress::new(40.0, 110.0),
        }
    }

    pub fn init(&mut self) {
        for panel in self.panels.iter_mut() {
            panel.init();
        }
        if self.no_data_tex.is_none() {
            self.no_data_tex = Some(vita2d_load_png_file(
                "ux0:app/SAVECLOUD/sce_sys/resources/no-data.png",
            ));
        }
        if self.qr_code_state.qr_code.is_some() {
            if Api::get_read().is_login() {
                self.qr_code_state.qr_code = None;
            }
            return;
        }
        // update qrcode Texture
        if let Ok(mut qr_code) = self.qr_code_state.qr_code_buf.try_write() {
            if let Some(buf) = &*qr_code {
                self.qr_code_state.qr_code = Some(vita2d_load_png_buf(buf));
                *qr_code = None;
            }
        }
        if Arc::strong_count(&self.qr_code_state.qr_code_buf) > 1 {
            return;
        }
        if !Api::get_read().is_login() {
            self.start_auth();
        }
    }

    fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }

    fn start_auth(&mut self) {
        let qr_code_buf = Arc::clone(&self.qr_code_state.qr_code_buf);
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
                        // 更新登录状态
                        match Api::start_fetch_name_of_pancake(
                            token_res.access_token.as_ref().unwrap(),
                        ) {
                            Ok(name_of_pancake) => {
                                Api::update_auth_data(
                                    api_type,
                                    Some(AuthData::new(token_res, name_of_pancake)),
                                );
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

    pub fn get_action_params(
        &mut self,
        from_path: &str,
        name: &str,
        to_path: &str,
    ) -> (
        Arc<RwLock<Option<DirPending>>>,
        Arc<RwLock<Option<DirPending>>>,
        String,
        String,
        String,
        String,
        String,
    ) {
        let from_panel = self.get_from_panel();
        let from_dir_pending_to_enter = Arc::clone(&from_panel.dir_pending_to_enter);
        let to_panel = self.get_to_panel();
        let to_dir_pending_to_enter = Arc::clone(&to_panel.dir_pending_to_enter);
        let from_path = from_path.to_string();
        let to_path = to_path.to_string();
        let from = join_path(&from_path, &name);
        let to = join_path(&to_path, &name);
        let name = name.to_string();

        (
            from_dir_pending_to_enter,
            to_dir_pending_to_enter,
            from_path,
            to_path,
            from,
            to,
            name,
        )
    }

    pub fn create_local_dir(&mut self, from_path: &str, to_path: &str) -> bool {
        let input = normalize_path(&show_keyboard(""));
        if input.is_empty() {
            return false;
        }
        match fs::create_dir(Path::new(&join_path(from_path, &input))) {
            Ok(_) => {
                self.get_from_panel().refresh_current_dir();
                if from_path == to_path {
                    self.get_to_panel().refresh_current_dir();
                }
                Toast::show("创建文件夹完成！".to_string());
            }
            Err(err) => {
                error!("create dir failed: {:?}", err);
                Toast::show(format!("创建文件夹失败：{}", err));
            }
        }

        true
    }

    pub fn create_cloud_dir(&mut self, from_path: &str, to_path: &str) -> bool {
        let input = normalize_path(&show_keyboard(""));
        if input.is_empty() {
            Toast::show("创建文件夹取消！".to_string());
            return false;
        }
        let (from_dir_pending_to_enter, _, from_path, _, _, _, name) =
            self.get_action_params(&from_path, "", to_path);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            Loading::notify_title("正在创建文件夹".to_string());
            Loading::notify_desc(input.clone());
            match Api::start_create_dir(&from_path, &input) {
                Ok(_) => {
                    do_cloud_action(
                        &from_path,
                        &name,
                        DirPendingAction::Refresh,
                        from_dir_pending_to_enter,
                    );
                    Toast::show("创建文件夹完成！".to_string());
                }
                Err(err) => {
                    error!("create dir failed: {:?}", err);
                    Toast::show(format!("创建文件夹失败：{}", err));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn rename_local(&mut self, from_path: &str, name: &str, to_path: &str) -> bool {
        let old_name = join_path(from_path, name);
        let input = normalize_path(&show_keyboard(name));
        if input.is_empty() {
            Toast::show("重命名取消！".to_string());
            return false;
        }
        let new_name = join_path(from_path, &input);
        if old_name == new_name {
            Toast::show("名字相同，重命名取消！".to_string());
            return false;
        }
        match fs::rename(old_name, new_name) {
            Ok(_) => {
                self.get_from_panel().refresh_current_dir();
                if from_path == to_path {
                    self.get_to_panel().refresh_current_dir();
                }
                Toast::show("重命名完成！".to_string());
            }
            Err(err) => {
                error!("rename failed: {:?}", err);
                Toast::show(format!("重命名失败：{}", err));
            }
        }

        true
    }

    pub fn rename_cloud(&mut self, from_path: &str, name: &str, to_path: &str) -> bool {
        let input = normalize_path(&show_keyboard(name));
        if input.is_empty() {
            Toast::show("重命名取消！".to_string());
            return false;
        }
        let (from_dir_pending_to_enter, _, from_path, _, from, _, name) =
            self.get_action_params(&from_path, name, to_path);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            Loading::notify_title("正在重命名".to_string());
            Loading::notify_desc(input.clone());
            match Api::start_file_manager(
                &from,
                None,
                Some(&input),
                crate::api::ApiOperates::Rename,
            ) {
                Ok(_) => {
                    do_cloud_action(
                        &from_path,
                        &name,
                        DirPendingAction::Refresh,
                        from_dir_pending_to_enter,
                    );
                    Toast::show("重命名完成！".to_string());
                }
                Err(err) => {
                    error!("rename failed: {:?}", err);
                    Toast::show(format!("重命名失败"));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn delete_local(
        &mut self,
        is_dir: bool,
        from_path: &str,
        name: &str,
        to_path: &str,
    ) -> bool {
        if !UIDialog::present(&format!("确定删除 {} ？", name)) {
            return false;
        }
        let (from_dir_pending_to_enter, to_dir_pending_to_enter, from_path, to_path, _, _, name) =
            self.get_action_params(from_path, name, to_path);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            let abs_path = join_path(&from_path, &name);
            match if is_dir {
                fs::remove_dir_all(abs_path)
            } else {
                fs::remove_file(abs_path)
            } {
                Ok(_) => {
                    do_local_action(
                        &from_path,
                        &name,
                        DirPendingAction::Refresh,
                        from_dir_pending_to_enter,
                    );
                    if from_path == to_path {
                        do_local_action(
                            &to_path,
                            &name,
                            DirPendingAction::Refresh,
                            to_dir_pending_to_enter,
                        );
                    }
                    Toast::show("删除完成！".to_string());
                }
                Err(err) => {
                    error!("remove failed: {:?}", err);
                    Toast::show(format!("删除失败：{}", err));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn delete_cloud(&mut self, from_path: &str, name: &str, to_path: &str) -> bool {
        if !UIDialog::present(&format!("确定删除 {} ？", name)) {
            return false;
        }
        let (from_dir_pending_to_enter, _, from_path, _, _, _, name) =
            self.get_action_params(&from_path, name, to_path);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            Loading::notify_title("正在删除文件".to_string());
            Loading::notify_desc(name.to_string());
            match Api::start_file_manager(
                &join_path(&from_path, &name),
                None,
                None,
                crate::api::ApiOperates::Delete,
            ) {
                Ok(_) => {
                    do_cloud_action(
                        &from_path,
                        &name,
                        DirPendingAction::Refresh,
                        from_dir_pending_to_enter,
                    );
                    Toast::show("删除完成！".to_string());
                }
                Err(err) => {
                    error!("delete failed: {:?}", err);
                    Toast::show(format!("删除失败"));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn copy_local(&mut self, is_dir: bool, from_path: &str, name: &str, to_path: &str) -> bool {
        if Path::new(&join_path(to_path, name)).exists() {
            Toast::show("目标文件已存在！".to_string());
            return false;
        }
        let (_, to_dir_pending_to_enter, _, to_path, from, to, name) =
            self.get_action_params(from_path, name, to_path);
        if to.starts_with(&from) {
            Toast::show("目标文件夹不能是源文件夹的子文件夹！".to_string());
            return false;
        }
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            match if is_dir {
                copy_dir_all(from, to)
            } else {
                fs::copy(from, to)
            } {
                Ok(_) => {
                    do_local_action(
                        &to_path,
                        &name,
                        DirPendingAction::Refresh,
                        to_dir_pending_to_enter,
                    );
                    Toast::show("复制完成！".to_string());
                }
                Err(err) => {
                    error!("copy failed: {:?}", err);
                    Toast::show(format!("复制失败：{}", err));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn move_local(&mut self, from_path: &str, name: &str, to_path: &str) -> bool {
        if Path::new(&join_path(to_path, &name)).exists() {
            Toast::show("目标文件已存在！".to_string());
            return false;
        }
        let (
            from_dir_pending_to_enter,
            to_dir_pending_to_enter,
            from_path,
            to_path,
            from,
            to,
            name,
        ) = self.get_action_params(from_path, name, to_path);
        if to.starts_with(&from) {
            Toast::show("目标文件夹不能是源文件夹的子文件夹！".to_string());
            return false;
        }
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            match fs::rename(from, to) {
                Ok(_) => {
                    do_local_action(
                        &from_path,
                        &name,
                        DirPendingAction::Refresh,
                        from_dir_pending_to_enter,
                    );
                    do_local_action(
                        &to_path,
                        &name,
                        DirPendingAction::Refresh,
                        to_dir_pending_to_enter,
                    );
                    Toast::show("移动完成！".to_string());
                }
                Err(err) => {
                    error!("copy failed: {:?}", err);
                    Toast::show(format!("移动失败：{}", err));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn zip_local(&mut self, is_dir: bool, from_path: &str, name: &str, to_path: &str) -> bool {
        let name_with_ext = format!("{}.zip", name);
        let output_path = join_path(from_path, &name_with_ext);
        let output_path = if !Path::new(&output_path).exists() {
            output_path
        } else {
            let input = normalize_path(&show_keyboard(&name));
            if input.is_empty() {
                Toast::show("压缩取消！".to_string());
                return false;
            }
            join_path(from_path, &format!("{}.zip", input))
        };
        if Path::new(&output_path).exists() {
            Toast::show("目标文件已存在！".to_string());
            return false;
        }
        let input_path = join_path(from_path, name);
        let (from_dir_pending_to_enter, to_dir_pending_to_enter, from_path, to_path, _, _, name) =
            self.get_action_params(from_path, name, to_path);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            Loading::notify_title("正在压缩".to_string());
            match if is_dir {
                zip_dir(&input_path, &output_path, &[])
            } else {
                zip_file(&from_path, &name, &output_path)
            } {
                Ok(_) => {
                    do_local_action(
                        &from_path,
                        &name,
                        DirPendingAction::Refresh,
                        from_dir_pending_to_enter,
                    );
                    if from_path == to_path {
                        do_local_action(
                            &to_path,
                            &name,
                            DirPendingAction::Refresh,
                            to_dir_pending_to_enter,
                        );
                    }
                    Toast::show("压缩完成！".to_string());
                }
                Err(err) => {
                    error!("zip failed: {:?}", err);
                    Toast::show(format!("压缩失败：{}", err));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn unzip_local(&mut self, from_path: &str, name: &str, to_path: &str) -> bool {
        // remove .zip ext name
        let name_without_ext = name[0..name.len() - 4].to_string();
        let output_dir = join_path(from_path, &name_without_ext);
        let output_dir = if !Path::new(&output_dir).exists() {
            output_dir
        } else {
            let input = normalize_path(&show_keyboard(&name_without_ext));
            if input.is_empty() {
                Toast::show("解压取消！".to_string());
                return false;
            }
            join_path(from_path, &input)
        };
        if Path::new(&output_dir).exists() {
            Toast::show("目标文件已存在！".to_string());
            return false;
        }
        let (from_dir_pending_to_enter, to_dir_pending_to_enter, from_path, to_path, _, _, name) =
            self.get_action_params(from_path, name, to_path);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            Loading::notify_title("正在解压".to_string());
            Loading::notify_desc(name.clone());
            match zip_extract(join_path(&from_path, &name), output_dir) {
                Ok(_) => {
                    do_local_action(
                        &from_path,
                        &name,
                        DirPendingAction::Refresh,
                        from_dir_pending_to_enter,
                    );
                    if from_path == to_path {
                        do_local_action(
                            &to_path,
                            &name,
                            DirPendingAction::Refresh,
                            to_dir_pending_to_enter,
                        );
                    }
                    Toast::show("解压完成！".to_string());
                }
                Err(err) => {
                    error!("zip failed: {:?}", err);
                    Toast::show(format!("解压失败：{}", err));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn upload_to_cloud(&mut self, from_path: &str, name: &str, to_path: &str) -> bool {
        let (_, to_dir_pending_to_enter, _, to_path, from, _, name) =
            self.get_action_params(&from_path, name, to_path);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            Loading::notify_title("正在上传".to_string());
            Loading::notify_desc(name.to_string());
            match Api::upload_to_cloud(&to_path, &name, &from, false) {
                Ok(_) => {
                    do_cloud_action(
                        &to_path,
                        &name,
                        DirPendingAction::Refresh,
                        to_dir_pending_to_enter,
                    );
                    Toast::show("上传完成！".to_string());
                }
                Err(err) => {
                    error!("upload failed: {:?}", err);
                    Toast::show(format!("上传失败"));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn download_from_cloud(
        &mut self,
        from_path: &str,
        name: &str,
        fs_id: u64,
        to_path: &str,
    ) -> bool {
        if Path::new(&join_path(to_path, name)).exists() {
            Toast::show("目标文件已存在！".to_string());
            return false;
        }
        let (_, to_dir_pending_to_enter, _, to_path, _, to, name) =
            self.get_action_params(&from_path, name, to_path);
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            Loading::notify_title("正在下载".to_string());
            Loading::notify_desc(name.to_string());
            match Api::start_download(fs_id, &to) {
                Ok(_) => {
                    do_local_action(
                        &to_path,
                        &name,
                        DirPendingAction::Refresh,
                        to_dir_pending_to_enter,
                    );
                    Toast::show("下载完成！".to_string());
                }
                Err(err) => {
                    error!("download failed: {:?}", err);
                    Toast::show(format!("下载失败"));
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn zip_local_and_upload(
        &mut self,
        is_dir: bool,
        from_path: &str,
        name: &str,
        to_path: &str,
    ) -> bool {
        let name_with_ext = format!("{}.zip", name);
        let tmp_name_with_ext = format!("{}.zip", get_current_format_time());
        let input_path = join_path(from_path, name);
        let (_, to_dir_pending_to_enter, from_path, to_path, output_path, _, _) =
            self.get_action_params(from_path, &tmp_name_with_ext, to_path);
        let name = name.to_string();
        let pending = Arc::clone(&self.pending);
        pending.store(true, Ordering::Relaxed);
        Loading::show();
        tokio::spawn(async move {
            Loading::notify_title("正在压缩".to_string());
            Loading::notify_desc(name.to_string());
            let is_success = match if is_dir {
                zip_dir(&input_path, &output_path, &[])
            } else {
                zip_file(&from_path, &name, &output_path)
            } {
                Ok(_) => true,
                Err(err) => {
                    error!("zip failed: {:?}", err);
                    Toast::show(format!("压缩失败：{}", err));
                    false
                }
            };
            if is_success {
                Loading::notify_title("正在上传".to_string());
                match Api::upload_to_cloud(&to_path, &name_with_ext, &output_path, false) {
                    Ok(_) => {
                        do_cloud_action(
                            &to_path,
                            &name,
                            DirPendingAction::Refresh,
                            to_dir_pending_to_enter,
                        );
                        Toast::show("上传完成！".to_string());
                    }
                    Err(err) => {
                        error!("upload failed: {:?}", err);
                        Toast::show(format!("上传失败：{}", err));
                    }
                }
            }
            if Path::new(&output_path).exists() {
                if let Err(err) = fs::remove_file(output_path) {
                    error!("remove tmp zip file failed: {:?}", err);
                }
            }
            pending.store(false, Ordering::Relaxed);
            Loading::hide();
        });

        true
    }

    pub fn get_from_panel(&mut self) -> &mut Panel {
        self.panels.get_mut(self.active_panel).unwrap()
    }

    pub fn get_to_panel(&mut self) -> &mut Panel {
        self.panels
            .get_mut(if self.active_panel == 0 {
                self.right_panel
            } else {
                0
            })
            .unwrap()
    }

    pub fn draw_current_dir_info(&self) {
        let left_panel = self.panels.get(0).unwrap();
        let right_panel = self.panels.get(self.right_panel).unwrap();
        let left_text = format!("本地：{}", &left_panel.current_dir_path());
        let right_text = format!(
            "{}：{}",
            if self.right_panel == 1 {
                "本地（右）"
            } else {
                "网盘"
            },
            &right_panel.current_dir_path()
        );
        let left = 330;
        // selected
        if let Some(dir) = if self.active_panel == 0 {
            left_panel.current_dir()
        } else {
            right_panel.current_dir()
        } {
            let current_position = format!("→ {}/{}", dir.state.selected_idx + 1, dir.items.len());
            vita2d_draw_text(
                left,
                60 + vita2d_text_height(1.0, &current_position),
                rgba(0xff, 0xff, 0xff, 0xff),
                1.0,
                &current_position,
            );
        }
        // left panel
        let mut text_width = vita2d_text_width(1.0, &left_text);
        let mut x = left;
        if text_width > SCREEN_WIDTH - 340 {
            vita2d_set_clip(x, 10, SCREEN_WIDTH - 10, 35);
            x = x
                - ((text_width - (SCREEN_WIDTH - 340)) as f32
                    * self.scroll_progress.progress() as f32) as i32;
        }
        vita2d_draw_text(
            x,
            10 + vita2d_text_height(1.0, &left_text),
            rgba(0xff, 0xff, 0xff, 0xff),
            1.0,
            &left_text,
        );
        if text_width > SCREEN_WIDTH - 340 {
            vita2d_unset_clip();
        }
        // right panel
        text_width = vita2d_text_width(1.0, &right_text);
        x = left;
        if text_width > SCREEN_WIDTH - 340 {
            vita2d_set_clip(x, 35, SCREEN_WIDTH - 10, 60);
            x = x
                - ((text_width - (SCREEN_WIDTH - 340)) as f32
                    * self.scroll_progress.progress() as f32) as i32;
        }
        vita2d_draw_text(
            x,
            35 + vita2d_text_height(1.0, &right_text),
            rgba(0xff, 0xff, 0xff, 0xff),
            1.0,
            &right_text,
        );
        if text_width > SCREEN_WIDTH - 340 {
            vita2d_unset_clip();
        }
    }
}

impl UIBase for UICloud {
    fn is_forces(&self) -> bool {
        self.panels.iter().any(|p| p.is_pending()) || self.is_pending() || self.menu.is_forces()
    }

    fn update(&mut self, app_data: &mut AppData, buttons: u32) {
        self.scroll_progress.update(buttons);

        self.init();

        if self.is_pending() {
            return;
        }

        // active menu
        if self.menu.is_forces() {
            if is_button(buttons, SceCtrlButtons::SceCtrlCircle) {
                let from_panel = self.panels.get(self.active_panel).unwrap();
                let to_panel = self
                    .panels
                    .get(if self.active_panel == 0 {
                        self.right_panel
                    } else {
                        0
                    })
                    .unwrap();
                let from_path = from_panel.current_dir_path();
                let to_path = to_panel.current_dir_path();
                let is_from_local = !from_path.starts_with("/");
                let item = from_panel.current_item();
                if item.is_none() {
                    match self.menu.get_selected_action().unwrap() {
                        menu::MenuAction::NewDir => {
                            if is_from_local {
                                self.create_local_dir(&from_path, &to_path);
                            } else {
                                if Api::is_eat_pancake_valid() {
                                    self.create_cloud_dir(&from_path, &to_path);
                                } else {
                                    UIDialog::present_qrcode(HOME_PAGE_URL);
                                }
                            }
                            self.menu.close();
                        }
                        _ => {}
                    }
                } else {
                    let item = item.unwrap();
                    let is_close_menu = match self.menu.get_selected_action().unwrap() {
                        menu::MenuAction::NewDir => {
                            if is_from_local {
                                self.create_local_dir(&from_path, &to_path)
                            } else {
                                if Api::is_eat_pancake_valid() {
                                    self.create_cloud_dir(&from_path, &to_path)
                                } else {
                                    UIDialog::present_qrcode(HOME_PAGE_URL);
                                    false
                                }
                            }
                        }
                        menu::MenuAction::Rename => {
                            if is_from_local {
                                self.rename_local(&from_path, &item.name.to_string(), &to_path)
                            } else {
                                if Api::is_eat_pancake_valid() {
                                    self.rename_cloud(&from_path, &item.name.to_string(), &to_path)
                                } else {
                                    UIDialog::present_qrcode(HOME_PAGE_URL);
                                    false
                                }
                            }
                        }
                        menu::MenuAction::Delete => {
                            if is_from_local {
                                self.delete_local(
                                    item.is_dir,
                                    &from_path,
                                    &item.name.to_string(),
                                    &to_path,
                                )
                            } else {
                                if Api::is_eat_pancake_valid() {
                                    self.delete_cloud(&from_path, &item.name.to_string(), &to_path)
                                } else {
                                    UIDialog::present_qrcode(HOME_PAGE_URL);
                                    false
                                }
                            }
                        }
                        menu::MenuAction::Copy => self.copy_local(
                            item.is_dir,
                            &from_path,
                            &item.name.to_string(),
                            &to_path,
                        ),
                        menu::MenuAction::Move => {
                            self.move_local(&from_path, &item.name.to_string(), &to_path)
                        }
                        menu::MenuAction::Unzip => {
                            self.unzip_local(&from_path, &item.name.to_string(), &to_path)
                        }
                        menu::MenuAction::Zip => self.zip_local(
                            item.is_dir,
                            &from_path,
                            &item.name.to_string(),
                            &to_path,
                        ),
                        menu::MenuAction::Upload => {
                            if Api::is_eat_pancake_valid() {
                                self.upload_to_cloud(&from_path, &item.name.to_string(), &to_path)
                            } else {
                                UIDialog::present_qrcode(HOME_PAGE_URL);
                                false
                            }
                        }
                        menu::MenuAction::ZipUpload => {
                            if Api::is_eat_pancake_valid() {
                                self.zip_local_and_upload(
                                    item.is_dir,
                                    &from_path,
                                    &item.name.to_string(),
                                    &to_path,
                                )
                            } else {
                                UIDialog::present_qrcode(HOME_PAGE_URL);
                                false
                            }
                        }
                        menu::MenuAction::Download => {
                            if Api::is_eat_pancake_valid() {
                                self.download_from_cloud(
                                    &from_path,
                                    &item.name.to_string(),
                                    item.fs_id.unwrap(),
                                    &to_path,
                                )
                            } else {
                                UIDialog::present_qrcode(HOME_PAGE_URL);
                                false
                            }
                        }
                        menu::MenuAction::ChangeAccountId => {
                            if let Some(path) = Path::new(&from_path).parent() {
                                mount_pfs(path.to_str().unwrap());
                                if let Ok(()) = update_sfo_file_with_current_account_id(&join_path(
                                    &from_path, &item.name,
                                )) {
                                    Toast::show("修改为当前账号完成！".to_string());
                                } else {
                                    Toast::show("修改为当前账号失败！".to_string());
                                }
                                unmount_pfs();
                            }
                            false
                        }
                    };
                    if is_close_menu {
                        self.menu.close();
                    }
                }
            } else {
                self.menu.update(buttons);
            }
            return;
        }

        // active panel
        let active_panel = self.panels.get_mut(self.active_panel).unwrap();
        active_panel.update(app_data, buttons);

        if active_panel.is_forces() {
            return;
        } else if is_button(buttons, SceCtrlButtons::SceCtrlLeft) {
            self.active_panel = 0
        } else if is_button(buttons, SceCtrlButtons::SceCtrlRight) {
            self.active_panel = self.right_panel
        } else if is_button(buttons, SceCtrlButtons::SceCtrlSquare) {
            self.right_panel = if self.right_panel == 2 { 1 } else { 2 };
            if self.active_panel != 0 {
                self.active_panel = self.right_panel;
            }
        } else if is_button(buttons, SceCtrlButtons::SceCtrlTriangle) {
            let panel = self.panels.get(self.active_panel).unwrap();
            let path = panel.current_dir_path();
            if path != "" {
                let panel_to = self
                    .panels
                    .get(if self.active_panel == 0 {
                        self.right_panel
                    } else {
                        0
                    })
                    .unwrap();
                self.menu
                    .open(panel.current_item(), &path, &panel_to.current_dir_path());
            } else {
                Toast::show("请选择文件夹或文件！".to_string());
            }
        }
    }

    fn draw(&self, _app_data: &AppData) {
        vita2d_line(
            (SCREEN_WIDTH / 2) as f32,
            90.0,
            (SCREEN_WIDTH / 2) as f32,
            (SCREEN_HEIGHT - 58) as f32,
            rgba(0x99, 0x99, 0x99, 0xff),
        );

        self.draw_current_dir_info();

        for (idx, panel) in self.panels.iter().enumerate() {
            if idx > 0 && idx != self.right_panel {
                continue;
            }

            panel.draw(idx == self.active_panel, &self.no_data_tex);

            if idx == 2 && self.qr_code_state.qr_code.is_some() {
                let left = SCREEN_WIDTH / 2;
                let x = (left + (SCREEN_WIDTH / 2 - SAVE_LIST_QR_CODE_SIZE) / 2) as f32;
                let y = 130.0;
                vita2d_draw_texture(self.qr_code_state.qr_code.as_ref().unwrap(), x, y);
                vita2d_draw_text(
                    left + (SCREEN_WIDTH / 2 - vita2d_text_width(1.0, SCAN_QR_CODE_TIPS)) / 2,
                    SCREEN_HEIGHT - 80,
                    rgba(0xff, 0xff, 0xff, 0xff),
                    1.0,
                    SCAN_QR_CODE_TIPS,
                )
            }
        }

        self.menu.draw();
    }
}
