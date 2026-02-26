#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod constants;

fn main() {
    app::run();
}
