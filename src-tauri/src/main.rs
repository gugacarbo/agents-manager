#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--export-bindings") {
        let output = agents_manager_lib::default_bindings_path();
        if let Err(err) = agents_manager_lib::export_bindings_to(&output) {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }

    agents_manager_lib::run();
}
