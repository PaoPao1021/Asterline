#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(result) = asterline_desktop_lib::run_attach_helper_from_args() {
        if let Err(error) = result {
            eprintln!("Asterline attach failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    asterline_desktop_lib::run();
}
