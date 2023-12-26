use std::{
    path::Path,
    sync::{Arc, RwLock},
};

use crate::{
    constant::PSV_DEVICES,
    ui::ui_cloud::panel::{Dir, DirPending, DirPendingAction, Item},
};

use super::{do_local_action, Action};

pub struct LocalAction {}

impl LocalAction {
    pub fn new() -> LocalAction {
        LocalAction {}
    }
}

impl Action for LocalAction {
    fn init(&mut self, dirs: &mut Vec<Dir>, _dir: &Arc<RwLock<Option<DirPending>>>) {
        if dirs.len() > 0 {
            return;
        }
        let mut dir = Dir::new("".to_string(), vec![]);
        for dev in PSV_DEVICES.iter() {
            if !Path::new(dev).exists() {
                continue;
            }
            dir.items.push(Item::new(true, dev.to_string(), None))
        }
        dirs.push(dir);
    }

    fn do_action(
        &self,
        path: &str,
        item_name: &str,
        action: DirPendingAction,
        dir: &Arc<RwLock<Option<DirPending>>>,
    ) {
        let dir = Arc::clone(dir);
        let path = path.to_string();
        let name = item_name.to_string();
        tokio::spawn(async move {
            do_local_action(&path, &name, action, dir);
        });
    }

    fn pop_dir(&self, dirs: &mut Vec<Dir>) {
        if dirs.len() <= 1 {
            return;
        }
        dirs.pop();
    }
}
