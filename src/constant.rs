/* console buttons symbol
 *
 * "〇△ □ X"
 */

use std::time::Duration;

pub const SCREEN_WIDTH: i32 = 960;
pub const SCREEN_HEIGHT: i32 = 544;

// psv game save path
pub const GAME_CARD_SAVE_DIR: &str = "grw0:savedata";
pub const GAME_SAVE_DIR: &str = "ux0:user/00/savedata";
pub const PSV_DEVICES: [&str; 11] = [
    "ux0:", "uma0:", "grw0:", "os0:", "pd0:", "sa0:", "tm0:", "ud0:", "ur0:", "vd0:", "vs0:",
];
// save cloud dir
pub const SAVE_CLOUD_DIR: &str = "ux0:data/save-cloud";
// save local dir
pub const GAME_SAVE_LOCAL_DIR: &str = "ux0:data/save-cloud/saves";
// save cloud prefix
pub const GAME_SAVE_CLOUD_DIR_PREFIX: &str = "/apps/Backup/";
// save cloud root dir
pub const GAME_SAVE_CLOUD_DIR_ROOT: &str = "/apps/Backup/psvita/save-cloud";
// save cloud dir
pub const GAME_SAVE_CLOUD_DIR: &str = "/apps/Backup/psvita/save-cloud/saves";
// update cache dir
pub const UPLOAD_CACHE_DIR: &str = "/apps/Backup/upload_cache_can_delete";
// log path
pub const SAVE_LOG_PATH: &str = "ux0:data/save-cloud/log/log.txt";
// baidu auth config path
pub const AUTH_BAIDU_CONFIG_PATH: &str = "ux0:data/save-cloud/auth";

// app
pub const BUTTON_HOLDING_DELAY: u128 = 360;
pub const BUTTON_HOLDING_REPEAT_DELAY: u128 = 60;
// desktop/titles
pub const TEXT_L: &str = "L ←";
pub const TEXT_R: &str = "→ R";
// desktop
pub const DESKTOP_BOTTOM_BAR_TEXT: &str = "(START) 退出    (□) 关于    (△) 存档    (〇) 备份/还原";
pub const DESKTOP_BOTTOM_BAR_CLOUD_TEXT: &str =
    "(START) 退出    (□) 切换    (△) 操作    (X) 返回    (〇) 选择";

// titles
pub const SAVE_DRAWER_BOTTOM_BAR_TEXT: &str =
    "(SELECT) 上传    (□) 还原    (△) 删除    (X) 关闭    (〇) 选择";
pub const SAVE_DRAWER_CLOUD_BOTTOM_BAR_TEXT: &str =
    "(SELECT) 下载    (□) 还原    (△) 删除    (X) 关闭    (〇) 选择";
pub const ACTION_DRAWER_BOTTOM_BAR_TEXT: &str = "(X) 关闭    (〇) 选择";
pub const TITLE_DRAWER_BOTTOM_BAR_TEXT: &str = "(X) 关闭    (〇) 选择";
pub const TAB_LOCAL: &str = "本地备份";
pub const TAB_CLOUD: &str = "云端备份";
pub const NEW_BACKUP: &str = "新建备份";
pub const NEW_CLOUD_BACKUP: &str = "新建云备份";
// ignore list
pub const BACKUP_BLACK_LIST: [&str; 4] = [
    "sce_pfs",
    "sce_sys/safemem.dat",
    "sce_sys/keystone",
    "sce_sys/sealedkey",
];
pub const ANIME_TIME_300: Duration = Duration::from_millis(300);
pub const ANIME_TIME_160: Duration = Duration::from_millis(160);

// save list
pub const SAVE_LIST_QR_CODE_SIZE: i32 = SCREEN_WIDTH / 2 - 180;
pub const SCAN_QR_CODE_TIPS: &str = "百度云 App 扫码登录";
pub const UPLOAD_SLICE_PER_SIZE: usize = 1024 * 1024 * 4; // 4 MiB
pub const DOWNLOAD_BUF_SIZE: usize = 1024 * 512; // 512 Kib
pub const LIST_NAME_WIDTH: i32 = SCREEN_WIDTH / 2 - 40;

// dialog
pub const DIALOG_WIDTH: i32 = 600;
pub const DIALOG_HEIGHT: i32 = 260;
pub const DIALOG_BOTTOM_TOP: i32 = 220;
pub const DIALOG_CONFIRM_TEXT: &str = "(〇) 确定";
pub const DIALOG_CANCEL_TEXT: &str = "(X) 取消";

// home page
pub const HOME_PAGE_URL: &str = "https://save-cloud.sketchraw.com?psvita=go";
pub const INVALID_EAT_PANCAKE: &str = "缺少 eat.pancake";
pub const ABOUT_TEXT: &str = "Save Cloud 云存档，扫码访问主页！";

// certificate
pub const SSL_CERT_ENV_KEY: &str = "SSL_CERT_FILE";
pub const CURL_CERT_CURL: &str = "https://curl.se/ca/cacert.pem";
pub const SAVE_CLOUD_CERT: &str = "ux0:data/save-cloud/cacert.pem";
pub const PSV_DEVICE_CERT: &str = "vs0:data/external/cert/CA_LIST.cer";
