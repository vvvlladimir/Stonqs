// No console window on Windows release builds: this is a windowed app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sq_app_lib::run();
}
