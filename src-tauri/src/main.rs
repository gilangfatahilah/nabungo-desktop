// Mencegah jendela konsol tambahan di Windows saat release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    nabungo_desktop_lib::run()
}
