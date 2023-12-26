pub trait UIList {
    fn init(&mut self);

    fn is_pending(&self) -> bool;

    fn do_restore_game_save(&self, _game_save_dir: &Option<String>, _backup_name: &str) {}

    fn do_backup_game_save(&self, game_save_dir: &Option<String>, input: Option<String>);

    fn do_delete_game_save(&self, backup_name: &str);

    fn update(&mut self, game_save_dir: &Option<String>, buttons: u32);

    fn draw(&self, left: i32, top: i32);
}
