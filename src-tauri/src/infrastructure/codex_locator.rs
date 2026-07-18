use std::{env, fs, path::Path, path::PathBuf, process::Stdio};

pub fn discover_codex_executable(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = configured
        .map(PathBuf::from)
        .and_then(expand_codex_candidate)
    {
        return Some(path);
    }

    let mut candidates = Vec::new();
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            candidates.push(directory.join("codex.exe"));
            candidates.push(directory.join("codex.cmd"));
            candidates.push(directory.join("codex"));
        }
    }
    if let Some(local) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        let alias = local
            .join("Microsoft")
            .join("WindowsApps")
            .join("codex.exe");
        if alias.is_file() {
            candidates.push(alias);
        }
        let bundled = local.join("OpenAI").join("Codex").join("bin");
        candidates.extend(
            fs::read_dir(bundled)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("codex.exe"))
                .filter(|path| path.is_file()),
        );
    }
    if let Some(program_files) = env::var_os("ProgramFiles").map(PathBuf::from) {
        candidates.extend(
            fs::read_dir(program_files.join("WindowsApps"))
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .starts_with("openai.codex_")
                })
                .map(|entry| entry.path().join("app").join("resources").join("codex.exe"))
                .filter(|path| path.is_file()),
        );
    }
    candidates
        .into_iter()
        .filter_map(expand_codex_candidate)
        .next()
}

fn expand_codex_candidate(path: PathBuf) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension == "cmd" || extension == "ps1" || extension.is_empty() {
        if let Some(native) = npm_native_codex(path.parent()?) {
            return command_available(&native).then_some(native);
        }
        return None;
    }
    command_available(&path).then_some(path)
}

fn npm_native_codex(prefix: &Path) -> Option<PathBuf> {
    let candidate = prefix
        .join("node_modules")
        .join("@openai")
        .join("codex")
        .join("node_modules")
        .join("@openai")
        .join("codex-win32-x64")
        .join("vendor")
        .join("x86_64-pc-windows-msvc")
        .join("bin")
        .join("codex.exe");
    candidate.is_file().then_some(candidate)
}

fn command_available(executable: &Path) -> bool {
    let mut command = std::process::Command::new(executable);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_std_window(&mut command);
    command.status().is_ok_and(|status| status.success())
}

#[cfg(windows)]
fn hide_std_window(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn hide_std_window(_command: &mut std::process::Command) {}

#[cfg(windows)]
pub fn hide_tokio_window(command: &mut tokio::process::Command) {
    use std::os::windows::process::CommandExt;
    command.as_std_mut().creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
pub fn hide_tokio_window(_command: &mut tokio::process::Command) {}
