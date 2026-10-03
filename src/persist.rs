//! Stores saved to disk (feature `persist`).
//!
//! A persisted store is a [`Store`] whose value is read from the app's config directory at
//! startup and written back, debounced, after it changes. Values are JSON with a version
//! number, so a newer app can migrate what an older one saved.
//!
//! ```no_run
//! use rok_ui::{gpui::App, persist};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Clone, Default, Serialize, Deserialize)]
//! struct Settings {
//!     dark_mode: bool,
//!     font_size: f32,
//! }
//!
//! fn init(cx: &mut App) {
//!     persist::set_app_name(cx, "notes");
//!     let settings = persist::persisted_store(cx, "settings", Settings::default);
//!     settings.update(|settings| settings.dark_mode = true); // saved shortly after
//! }
//! ```

use std::{
    any::Any,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use gpui::{App, Global};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::state::{create_store, Store};

type Migration = Rc<dyn Fn(u32, Value) -> Value>;

/// Where and how a store is saved.
#[derive(Clone)]
pub struct PersistOptions {
    key: String,
    version: u32,
    migrate: Option<Migration>,
    directory: Option<PathBuf>,
    debounce: Duration,
}

impl PersistOptions {
    /// Save under `key` (the file name, `<key>.json`).
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            version: 1,
            migrate: None,
            directory: None,
            debounce: Duration::from_millis(300),
        }
    }

    /// The format version written with the value. Default: 1.
    #[must_use]
    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Upgrade a value saved by an older version: called with the saved version and JSON, and
    /// returns JSON in the current format.
    #[must_use]
    pub fn migrate(mut self, migrate: impl Fn(u32, Value) -> Value + 'static) -> Self {
        self.migrate = Some(Rc::new(migrate));
        self
    }

    /// Save in `directory` instead of the app's config directory (for tests and portable
    /// installs).
    #[must_use]
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// How long a store must be still before it is written. Default: 300 ms.
    #[must_use]
    pub fn debounce(mut self, debounce: Duration) -> Self {
        self.debounce = debounce;
        self
    }

    fn path(&self, cx: &App) -> PathBuf {
        let directory = self.directory.clone().unwrap_or_else(|| {
            config_dir().join(
                cx.try_global::<AppName>()
                    .map_or_else(default_app_name, |name| name.0.clone()),
            )
        });
        directory.join(format!("{}.json", self.key))
    }
}

struct AppName(String);

impl Global for AppName {}

/// Keeps the change subscriptions of persisted stores alive.
#[derive(Default)]
struct Persisted(Vec<Box<dyn Any>>);

impl Global for Persisted {}

/// Name the folder the app's stores are saved in, inside the platform config directory.
/// Default: the executable's name.
pub fn set_app_name(cx: &mut App, name: impl Into<String>) {
    cx.set_global(AppName(name.into()));
}

fn default_app_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "rok-ui-app".into())
}

/// The platform config directory: `$XDG_CONFIG_HOME` or `~/.config` on Linux,
/// `~/Library/Application Support` on macOS, `%APPDATA%` on Windows.
#[must_use]
pub fn config_dir() -> PathBuf {
    let home = || {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
    };
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map_or_else(home, PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().join("Library/Application Support")
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map_or_else(|| home().join(".config"), PathBuf::from)
    }
}

/// A store saved as `<config dir>/<app name>/<key>.json`, starting from the saved value or
/// `default`.
pub fn persisted_store<T>(cx: &mut App, key: &str, default: impl FnOnce() -> T) -> Store<T>
where
    T: Serialize + DeserializeOwned + Clone + 'static,
{
    persisted_store_with(cx, PersistOptions::new(key), default)
}

/// A persisted store with options: a version and migration, a directory, a debounce.
pub fn persisted_store_with<T>(
    cx: &mut App,
    options: PersistOptions,
    default: impl FnOnce() -> T,
) -> Store<T>
where
    T: Serialize + DeserializeOwned + Clone + 'static,
{
    let path = options.path(cx);
    let initial = load(&path, &options).unwrap_or_else(default);
    let store = create_store(initial);
    let (version, debounce) = (options.version, options.debounce);
    let async_app = cx.to_async();
    let generation = Rc::new(std::cell::Cell::new(0_u64));
    let subscription = store.subscribe(move |value, _| {
        let value = value.clone();
        let path = path.clone();
        let generation = generation.clone();
        let run = generation.get() + 1;
        generation.set(run);
        let executor = async_app.background_executor().clone();
        async_app
            .foreground_executor()
            .spawn(async move {
                executor.timer(debounce).await;
                if generation.get() == run {
                    save(&path, version, &value).ok();
                }
            })
            .detach();
    });
    cx.default_global::<Persisted>()
        .0
        .push(Box::new(subscription));
    store
}

fn load<T: DeserializeOwned>(path: &Path, options: &PersistOptions) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    let saved: Value = serde_json::from_str(&text).ok()?;
    let saved_version = u32::try_from(saved.get("version")?.as_u64()?).ok()?;
    let mut value = saved.get("value")?.clone();
    if saved_version != options.version {
        value = (options.migrate.as_ref()?)(saved_version, value);
    }
    serde_json::from_value(value).ok()
}

fn save<T: Serialize>(path: &Path, version: u32, value: &T) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    let document = serde_json::json!({ "version": version, "value": value });
    let text = serde_json::to_string_pretty(&document).map_err(std::io::Error::other)?;
    // Write a sibling file and rename it, so a crash never leaves half a file.
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Settings {
        font_size: u32,
        theme: String,
    }

    fn directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("rok-ui-persist-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&directory).ok();
        directory
    }

    #[gpui::test]
    fn stores_save_after_changes_and_load_at_startup(cx: &mut gpui::TestAppContext) {
        let directory = directory("roundtrip");
        let options = PersistOptions::new("settings")
            .directory(&directory)
            .debounce(Duration::from_millis(50));
        let store = cx.update(|cx| persisted_store_with(cx, options.clone(), Settings::default));
        store.update(|settings| settings.font_size = 14);
        store.update(|settings| settings.theme = "dark".into());
        cx.executor().advance_clock(Duration::from_millis(60));
        cx.run_until_parked();

        let loaded = cx.update(|cx| persisted_store_with(cx, options, Settings::default));
        assert_eq!(
            loaded.get(),
            Settings {
                font_size: 14,
                theme: "dark".into()
            }
        );
        std::fs::remove_dir_all(directory).ok();
    }

    #[test]
    fn older_versions_are_migrated() {
        let directory = directory("migrate");
        let path = directory.join("settings.json");
        save(&path, 1, &serde_json::json!({ "size": 12 })).unwrap();
        let options = PersistOptions::new("settings")
            .directory(&directory)
            .version(2)
            .migrate(|from, old| {
                assert_eq!(from, 1);
                serde_json::json!({ "font_size": old["size"], "theme": "light" })
            });
        let loaded: Settings = load(&path, &options).unwrap();
        assert_eq!(
            loaded,
            Settings {
                font_size: 12,
                theme: "light".into()
            }
        );
        // Without a migration, an old file is ignored and the default is used.
        assert!(load::<Settings>(&path, &PersistOptions::new("settings").version(2)).is_none());
        std::fs::remove_dir_all(directory).ok();
    }
}
