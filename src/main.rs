use log::error;
use vita_save_cloud::app::App;
use vita_save_cloud::constant::SAVE_LOG_PATH;
use vita_save_cloud::log;
use vita_save_cloud::tai::{tai_init, Titles};
use vita_save_cloud::vita2d::Vita2dContext;

// stack
#[used]
#[export_name = "sceUserMainThreadStackSize"]
pub static SCE_USER_MAIN_THREAD_STACK_SIZE: u32 = 1 * 1024 * 1024; // 1 MiB

pub fn main() {
    let _log = log::open(SAVE_LOG_PATH)
        .size(100 * 1024)
        .rotate(10)
        .tee(if cfg!(debug_assertions) { true } else { false })
        .start();

    tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            // data
            let mut app = App::new(Titles::new());

            // vita2d init
            let _vita_ctx = Vita2dContext::new();

            // init tai
            let _tai = match tai_init() {
                Ok(tai) => tai,
                Err(err) => {
                    error!("modules init failed: {:?}", err);
                    return;
                }
            };

            app.present();
        });
}
