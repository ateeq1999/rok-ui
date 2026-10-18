//! Hydrated blocs and cubits (features `bloc` and `persist`): their state is saved after it
//! changes and restored when the app starts again.

use futures::{channel::mpsc, StreamExt as _};
use gpui::{App, Task};
use rok_ui_bloc::{Observable, Subscription};
use serde::{de::DeserializeOwned, Serialize};

use crate::persist::{DraftFile, PersistOptions};

/// A bloc or cubit whose state survives a restart: provided with
/// [`BlocProvider::with_hydrated_bloc`](super::BlocProvider::with_hydrated_bloc) (or
/// `with_hydrated_cubit`), it starts from the state saved by the last run and saves its state,
/// debounced, after each change.
///
/// The state is saved as JSON with a version, through [`persist`](crate::persist): when its
/// shape changes, raise the version and migrate older files. A saved state that cannot be read
/// (a newer version, or an older one without a migration) is skipped with a warning on
/// standard error, and the bloc starts from its initial state.
///
/// ```no_run
/// use rok_ui::bloc::{Bloc, Emitter, Hydrated};
/// use rok_ui::persist::PersistOptions;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
/// struct Settings {
///     dark_mode: bool,
/// }
///
/// #[derive(Debug)]
/// enum SettingsEvent {
///     DarkModeToggled,
/// }
///
/// struct SettingsBloc;
///
/// impl Bloc for SettingsBloc {
///     type Event = SettingsEvent;
///     type State = Settings;
///
///     fn initial_state(&self) -> Settings {
///         Settings::default()
///     }
///
///     async fn on(&self, event: SettingsEvent, emit: &Emitter<Settings>) {
///         match event {
///             SettingsEvent::DarkModeToggled => emit.update(|s| s.dark_mode = !s.dark_mode),
///         };
///     }
/// }
///
/// impl Hydrated for SettingsBloc {
///     fn persist(&self) -> PersistOptions {
///         PersistOptions::new("settings")
///     }
/// }
/// ```
pub trait Hydrated {
    /// Where and how the state is saved: its key (the file name), and a version with a
    /// migration once the state's shape changes.
    fn persist(&self) -> PersistOptions;
}

/// The state `file` holds, warning when it holds one that cannot be read.
pub(super) fn restore<S: DeserializeOwned>(file: &DraftFile, name: &str) -> Option<S> {
    let state = file.load::<S>();
    if state.is_none() && file.path().exists() {
        eprintln!(
            "rok-ui: the saved state of `{name}` in {} could not be read (a version without a \
             migration, or a different shape); starting from its initial state",
            file.path().display()
        );
    }
    state
}

/// Save `source`'s state to `file` after each change (debounced), while the returned
/// subscription and task are kept.
pub(super) fn save_changes<O>(source: &O, file: DraftFile, cx: &mut App) -> (Subscription, Task<()>)
where
    O: Observable,
    O::State: Serialize,
{
    let (sender, mut changes) = mpsc::unbounded::<O::State>();
    // Listeners run wherever the state changes (the bloc runtime); writes start on the main
    // thread.
    let subscription = source.subscribe(move |state| {
        sender.unbounded_send(state.clone()).ok();
    });
    let task = cx.spawn(async move |cx| {
        while let Some(mut state) = changes.next().await {
            // Of several changes that arrived together, only the last is saved.
            while let Ok(newer) = changes.try_recv() {
                state = newer;
            }
            if cx.update(|cx| file.save(state, cx)).is_err() {
                return;
            }
        }
    });
    (subscription, task)
}
