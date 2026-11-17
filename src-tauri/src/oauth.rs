//! App glue for Sign in with ChatGPT (`plexustyping_ai::chatgpt_auth`): where
//! the private credential file lives and how the system browser is opened.
//! The Tauri commands and the settings UI come later (T3).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use plexustyping_ai::Endpoints;
use plexustyping_ai::chatgpt_auth::{AuthError, CREDENTIALS_FILE, ChatgptAuth, Status};

/// `<app config dir>/chatgpt-auth.json`.
pub fn credentials_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CREDENTIALS_FILE)
}

/// The production sign-in, keeping its record in `config_dir` (none: nothing is stored).
pub fn chatgpt_auth(config_dir: Option<&Path>) -> ChatgptAuth {
    ChatgptAuth::new(&Endpoints::production(), config_dir.map(credentials_path))
}

/// The program and arguments that open `url` in the default browser on `os`.
/// No shell is involved, so `&` in the URL needs no quoting.
fn browser_command(os: &str, url: &str) -> (&'static str, Vec<String>) {
    match os {
        "macos" => ("open", vec![url.to_string()]),
        "windows" => (
            "rundll32",
            vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()],
        ),
        _ => ("xdg-open", vec![url.to_string()]),
    }
}

/// Opens `url` in the system browser. The error never repeats the URL (it may
/// carry an ID-token hint).
pub fn open_browser(url: &str) -> Result<(), String> {
    let (program, args) = browser_command(std::env::consts::OS, url);
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("{program}: {}", err.kind()))?;
    // Reaped in the background so no zombie process is left behind.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Signs in through the system browser; resolves once the browser comes back.
pub async fn sign_in(auth: &ChatgptAuth) -> Result<Status, AuthError> {
    auth.sign_in(open_browser).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_lives_in_the_app_config_dir() {
        let dir = Path::new("/home/u/.config/com.plexustyping.app");
        assert_eq!(
            credentials_path(dir),
            dir.join("chatgpt-auth.json"),
            "one private file, not the keychain"
        );
    }

    #[test]
    fn the_browser_is_opened_without_a_shell() {
        let url = "https://auth.openai.com/api/accounts/authorize?a=1&b=2";
        assert_eq!(
            browser_command("linux", url),
            ("xdg-open", vec![url.to_string()])
        );
        assert_eq!(
            browser_command("macos", url),
            ("open", vec![url.to_string()])
        );
        assert_eq!(
            browser_command("windows", url),
            (
                "rundll32",
                vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()]
            )
        );
    }

    #[tokio::test]
    async fn without_a_config_dir_nothing_is_stored() {
        let auth = chatgpt_auth(None);
        assert_eq!(auth.callback_port(), 1455);
        assert_eq!(auth.status().await.unwrap_err(), AuthError::NoStorage);
    }
}
