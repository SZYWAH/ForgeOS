#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|argument| argument == "--cleanup-owned-codex-mcp") {
        forgeos_lib::cleanup_owned_codex_mcp();
        return;
    }
    forgeos_lib::run();
}
