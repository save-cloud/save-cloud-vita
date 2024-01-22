use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crate::{
    api::Api,
    ui::{
        ui_cloud::panel::{Dir, DirPending, DirPendingAction},
        ui_loading::Loading,
    },
};

const INIT_RETRY_DURATION: Duration = Duration::from_millis(1000 * 10);

use super::{do_cloud_action, Action};

pub struct CloudAction {
    last_init_at: Instant,
}

impl CloudAction {
    pub fn new() -> CloudAction {
        CloudAction {
            last_init_at: Instant::now() - INIT_RETRY_DURATION,
        }
    }
}

impl Action for CloudAction {
    fn init(&mut self, dirs: &mut Vec<Dir>, dir: &Arc<RwLock<Option<DirPending>>>) {
        if dirs.len() > 0 {
            return;
        }
        if !Api::get_read().is_login() {
            return;
        }
        if Instant::now() - self.last_init_at < INIT_RETRY_DURATION {
            return;
        }
        self.last_init_at = Instant::now();
        self.do_action("", "/", DirPendingAction::Enter, dir);
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
        Loading::show();
        tokio::spawn(async move {
            do_cloud_action(&path, &name, action, dir);
            Loading::hide();
        });
    }

    fn pop_dir(&self, dirs: &mut Vec<Dir>) {
        if dirs.len() <= 1 {
            return;
        }
        dirs.pop();
    }
}
