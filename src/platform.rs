//! Small platform helpers: the clipboard, and saving bytes where the user chooses.

use std::{io, path::PathBuf};

use gpui::{App, ClipboardItem, Task};

/// Put `text` on the clipboard (a "Copy" button, copying a link or an id).
pub fn copy_to_clipboard(text: impl Into<String>, cx: &App) {
    cx.write_to_clipboard(ClipboardItem::new_string(text.into()));
}

/// Ask where to save `bytes` (the platform's save dialog, opening in the Downloads folder and
/// suggesting `suggested_name`), then write them there off the main thread. The task ends with
/// the path written, or `None` when the user cancelled.
///
/// ```no_run
/// # fn export(report: Vec<u8>, cx: &mut rok_ui::gpui::App) {
/// rok_ui::platform::save_bytes(report, "report.csv", cx).detach();
/// # }
/// ```
///
/// # Errors
///
/// The dialog could not be opened (possible on Linux without a portal), or writing failed.
pub fn save_bytes(
    bytes: impl Into<Vec<u8>>,
    suggested_name: &str,
    cx: &App,
) -> Task<io::Result<Option<PathBuf>>> {
    let bytes = bytes.into();
    let chosen = cx.prompt_for_new_path(&start_directory(), Some(suggested_name));
    let background = cx.background_executor().clone();
    cx.spawn(async move |_| {
        // A dropped dialog counts as cancelled.
        let Ok(chosen) = chosen.await else {
            return Ok(None);
        };
        let Some(path) = chosen.map_err(|error| io::Error::other(error.to_string()))? else {
            return Ok(None);
        };
        let target = path.clone();
        background
            .spawn(async move { std::fs::write(target, bytes) })
            .await?;
        Ok(Some(path))
    })
}

/// Where the save dialog opens: the Downloads folder, else the home folder, else the current
/// directory.
fn start_directory() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    match home {
        Some(home) if home.join("Downloads").is_dir() => home.join("Downloads"),
        Some(home) => home,
        None => std::env::current_dir().unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;

    #[gpui::test]
    fn copied_text_lands_on_the_clipboard(cx: &mut TestAppContext) {
        cx.update(|cx| copy_to_clipboard("hello", cx));
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some("hello".to_string())
        );
    }

    #[gpui::test]
    fn saved_bytes_go_where_the_user_chose(cx: &mut TestAppContext) {
        let target = std::env::temp_dir().join(format!("rok-ui-save-{}.csv", std::process::id()));
        let task = cx.update(|cx| save_bytes(b"a,b\n".to_vec(), "report.csv", cx));
        cx.simulate_new_path_selection(|_| Some(target.clone()));
        cx.run_until_parked();
        let saved = cx.background_executor.block_test(task).unwrap();
        assert_eq!(saved.as_deref(), Some(target.as_path()));
        assert_eq!(std::fs::read(&target).unwrap(), b"a,b\n");
        std::fs::remove_file(&target).ok();

        // Cancelling the dialog saves nothing.
        let task = cx.update(|cx| save_bytes(b"x".to_vec(), "report.csv", cx));
        cx.simulate_new_path_selection(|_| None);
        cx.run_until_parked();
        assert_eq!(cx.background_executor.block_test(task).unwrap(), None);
    }
}
