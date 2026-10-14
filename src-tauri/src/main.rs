#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod desktop;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(debug_assertions)]
mod probe;
mod surfaces;
mod tasks;

fn main() {
    desktop::run().expect("Impossible de démarrer Mystline");
}
