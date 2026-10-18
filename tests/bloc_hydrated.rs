//! Hydrated blocs and cubits: their state is saved after changes and restored by the next run.

// Test blocs emit without awaiting anything; `Bloc::on` is async regardless.
#![allow(clippy::unused_async_trait_impl)]

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use gpui::{TestAppContext, VisualTestContext};
use rok_ui::{
    bloc::{Bloc, BlocHandle, BlocProvider, Cubit, CubitHandle, Emitter, Hydrated, Observable},
    persist::PersistOptions,
    prelude::*,
};
use serde::{Deserialize, Serialize};

const DEBOUNCE: Duration = Duration::from_millis(20);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Settings {
    font_size: u32,
}

#[derive(Debug)]
enum SettingsEvent {
    Grown,
}

/// Saves its settings in `directory`.
struct SettingsBloc {
    directory: PathBuf,
    version: u32,
}

impl Bloc for SettingsBloc {
    type Event = SettingsEvent;
    type State = Settings;

    fn initial_state(&self) -> Settings {
        Settings { font_size: 12 }
    }

    async fn on(&self, event: SettingsEvent, emit: &Emitter<Settings>) {
        match event {
            SettingsEvent::Grown => emit.update(|settings| settings.font_size += 2),
        };
    }
}

impl Hydrated for SettingsBloc {
    fn persist(&self) -> PersistOptions {
        PersistOptions::new("settings")
            .directory(&self.directory)
            .version(self.version)
            .debounce(DEBOUNCE)
    }
}

/// A counter cubit saved in `directory`.
struct TabCubit {
    emitter: Emitter<usize>,
    directory: PathBuf,
}

impl TabCubit {
    fn select(&self, tab: usize) {
        self.emitter.emit(tab);
    }
}

impl Cubit for TabCubit {
    type State = usize;

    fn emitter(&self) -> &Emitter<usize> {
        &self.emitter
    }
}

impl Hydrated for TabCubit {
    fn persist(&self) -> PersistOptions {
        PersistOptions::new("tab")
            .directory(&self.directory)
            .debounce(DEBOUNCE)
    }
}

type Seen = Rc<RefCell<Option<(BlocHandle<SettingsBloc>, CubitHandle<TabCubit>)>>>;

#[component]
fn Reader(seen: Seen, cx: &mut Cx) -> impl IntoElement {
    *seen.borrow_mut() = Some((cx.bloc::<SettingsBloc>(), cx.cubit::<TabCubit>()));
    div()
}

struct App {
    directory: PathBuf,
    version: u32,
    seen: Seen,
}

impl Render for App {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (directory, version) = (self.directory.clone(), self.version);
        let cubit_directory = self.directory.clone();
        BlocProvider::new()
            .with_hydrated_bloc(move |_| SettingsBloc { directory, version })
            .with_hydrated_cubit(move |_| TabCubit {
                emitter: Emitter::new(0),
                directory: cubit_directory,
            })
            .child(Reader::new(self.seen.clone()))
    }
}

/// A run of the app: a window over `directory`, with the handles its view read.
fn run<'a>(
    cx: &'a mut TestAppContext,
    directory: &Path,
    version: u32,
) -> (
    BlocHandle<SettingsBloc>,
    CubitHandle<TabCubit>,
    &'a mut VisualTestContext,
) {
    let seen = Seen::default();
    let (view_seen, directory) = (seen.clone(), directory.to_path_buf());
    let (_, window) = cx.add_window_view(move |_, _| App {
        directory: directory.clone(),
        version,
        seen: view_seen.clone(),
    });
    window.run_until_parked();
    let (bloc, cubit) = seen.borrow().clone().expect("rendered");
    (bloc, cubit, window)
}

/// Let the bloc finish, the change reach the main thread, and the debounced write land.
fn save(cx: &mut VisualTestContext, bloc: &BlocHandle<SettingsBloc>) {
    rok_ui::runtime::block_on(bloc.idle());
    std::thread::sleep(Duration::from_millis(10));
    cx.run_until_parked();
    cx.executor().advance_clock(DEBOUNCE * 2);
    cx.run_until_parked();
}

fn directory(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("rok-ui-hydrated-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&directory).ok();
    directory
}

fn setup(cx: &mut TestAppContext) {
    cx.update(rok_ui::init);
    cx.executor().allow_parking();
}

#[gpui::test]
fn state_survives_a_restart(cx: &mut TestAppContext) {
    setup(cx);
    let directory = directory("restart");
    {
        let (bloc, cubit, window) = run(cx, &directory, 1);
        assert_eq!(
            bloc.state(),
            Settings { font_size: 12 },
            "nothing saved yet"
        );
        assert_eq!(cubit.state(), 0);
        bloc.add(SettingsEvent::Grown);
        bloc.add(SettingsEvent::Grown);
        cubit.select(3);
        save(window, &bloc);
    }
    assert!(directory.join("settings.json").exists());
    assert!(directory.join("tab.json").exists());

    let (bloc, cubit, _) = run(cx, &directory, 1);
    assert_eq!(bloc.state(), Settings { font_size: 16 });
    assert_eq!(cubit.state(), 3);
    std::fs::remove_dir_all(directory).ok();
}

#[gpui::test]
fn an_unreadable_state_starts_from_the_initial_one(cx: &mut TestAppContext) {
    setup(cx);
    let directory = directory("unreadable");
    {
        let (bloc, _, window) = run(cx, &directory, 1);
        bloc.add(SettingsEvent::Grown);
        save(window, &bloc);
    }
    // Version 2 has no migration from version 1.
    let (bloc, _, window) = run(cx, &directory, 2);
    assert_eq!(bloc.state(), Settings { font_size: 12 });
    // The next change saves over the old file, in the new version.
    bloc.add(SettingsEvent::Grown);
    save(window, &bloc);
    let saved = std::fs::read_to_string(directory.join("settings.json")).unwrap();
    assert!(saved.contains("\"version\": 2"), "{saved}");
    std::fs::remove_dir_all(directory).ok();
}
