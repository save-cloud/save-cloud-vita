use std::{
    path::Path,
    sync::{Arc, RwLock},
};

use log::error;

use crate::{api::Api, ui::ui_toast::Toast, utils::join_path};

use super::panel::{Dir, DirPending, DirPendingAction, Item};

pub mod cloud;
pub mod local;

pub trait Action {
    fn init(&mut self, dirs: &mut Vec<Dir>, dir: &Arc<RwLock<Option<DirPending>>>);

    fn do_action(
        &self,
        path: &str,
        item_name: &str,
        action: DirPendingAction,
        dir: &Arc<RwLock<Option<DirPending>>>,
    );

    fn pop_dir(&self, dirs: &mut Vec<Dir>);
}

pub fn do_local_action(
    path: &str,
    item_name: &str,
    action: DirPendingAction,
    dir: Arc<RwLock<Option<DirPending>>>,
) {
    let (abs_path, name) = get_path_and_name(path, item_name, &action);
    let mut dir_new = Dir::new(name, vec![]);
    match Path::new(&abs_path).read_dir() {
        Ok(read_dir) => {
            for entry in read_dir {
                if entry.is_err() {
                    continue;
                }
                let entry = entry.unwrap();
                match entry.file_type() {
                    Ok(file_type) => {
                        dir_new.items.push(Item::new(
                            file_type.is_dir(),
                            entry.file_name().to_string_lossy().to_string(),
                            None,
                        ));
                    }
                    Err(err) => {
                        error!("read dir error: {:?}", err);
                    }
                }
            }
        }
        Err(err) => {
            error!("read dir error: {:?}", err);
            Toast::show("切换目录发生错误！".to_string());
        }
    }
    dir_new.items.sort_by(|a, b| {
        if a.is_dir && !b.is_dir {
            return std::cmp::Ordering::Less;
        } else if !a.is_dir && b.is_dir {
            return std::cmp::Ordering::Greater;
        }
        a.name.to_uppercase().cmp(&b.name.to_uppercase())
    });
    *dir.write().expect("get dir write lock") = Some(DirPending {
        action,
        dir: dir_new,
    });
}

pub fn do_cloud_action(
    path: &str,
    item_name: &str,
    action: DirPendingAction,
    dir: Arc<RwLock<Option<DirPending>>>,
) {
    let (abs_path, name) = get_path_and_name(path, item_name, &action);
    let api_type = Api::get_read().api_type;
    let url = Api::get_read().get_file_list_url(&abs_path, 0);
    match Api::start_fetch_dir_list(&url, api_type) {
        Ok(list) => {
            let mut dir_new = Dir::new(name, vec![]);
            for item in list {
                dir_new.add_item(item.isdir == 1, item.server_filename, Some(item.fs_id));
            }
            *dir.write().expect("get dir write lock") = Some(DirPending {
                action,
                dir: dir_new,
            });
        }
        Err(err) => {
            error!("fetch {} list failed: {}", abs_path, err);
            Toast::show("获取列表失败".to_string());
        }
    }
}

pub fn get_path_and_name(
    path: &str,
    item_name: &str,
    action: &DirPendingAction,
) -> (String, String) {
    match action {
        DirPendingAction::Enter => (join_path(path, item_name), item_name.to_string()),
        DirPendingAction::Refresh => {
            if path == "/" || path.ends_with(":") {
                (path.to_string(), path.to_string())
            } else {
                (
                    path.to_string(),
                    Path::new(path)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                )
            }
        }
    }
}
