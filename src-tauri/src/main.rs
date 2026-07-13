#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cursor_skin_manager_lib::install_startup_diagnostics();
    cursor_skin_manager_lib::run()
}
