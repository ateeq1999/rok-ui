//! A PostgreSQL-backed user list with rok-db.
//!
//! Run with a database URL:
//!
//! ```sh
//! DATABASE_URL=postgres://user:password@localhost/app cargo run --example db_users --features db
//! ```
//!
//! The example creates a `rok_ui_example_users` table if it is missing, lists its
//! rows with `db::use_query`, and inserts and deletes with `db::run`, invalidating
//! the query afterwards.

use rok_ui::db::{self, rok_db::prelude::*};
use rok_ui::prelude::*;

#[derive(Debug, Clone, Model)]
#[rok(crate = "rok_ui::db::rok_db", table = "rok_ui_example_users")]
struct User {
    #[rok(primary_key, generated)]
    id: i64,
    email: String,
    name: Option<String>,
}

const CREATE_TABLE: &str = "CREATE TABLE IF NOT EXISTS rok_ui_example_users (
    id BIGSERIAL PRIMARY KEY,
    email TEXT NOT NULL,
    name TEXT
)";

/// Run `write`, then refresh the user list.
fn write_then_refresh<Fut>(cx: &mut App, write: impl FnOnce(Db) -> Fut)
where
    Fut: std::future::Future<Output = rok_ui::db::rok_db::Result<()>> + Send + 'static,
{
    let task = db::run(cx, write);
    cx.spawn(async move |cx| {
        let result = task.await;
        cx.update(|cx| {
            if let Err(error) = result {
                toast(
                    cx,
                    Toast::error("Could not save").description(error.to_string()),
                );
            }
            db::invalidate("users", cx);
        })
        .ok();
    })
    .detach();
}

#[component]
fn UsersCard(window: &mut Window, cx: &mut App) -> impl IntoElement {
    let users = db::use_query("users", window, cx, |db| async move {
        User::query().order_by(User::ID.asc()).all(&db).await
    });
    let email = use_input_state("email", window, cx, |state| {
        state.with_placeholder("ada@example.com")
    });
    let name = use_input_state("name", window, cx, |state| {
        state.with_placeholder("Ada Lovelace")
    });

    let add = {
        let (email, name) = (email.clone(), name.clone());
        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            let email_text = email.read(cx).text().trim().to_string();
            if email_text.is_empty() {
                return;
            }
            let name_text = name.read(cx).text().trim().to_string();
            let user = User {
                id: 0,
                email: email_text,
                name: (!name_text.is_empty()).then_some(name_text),
            };
            write_then_refresh(
                cx,
                move |db| async move { user.insert(&db).await.map(|_| ()) },
            );
            email.update(cx, |state, cx| state.set_text("", cx));
            name.update(cx, |state, cx| state.set_text("", cx));
        }
    };

    let body = match (users.data(), users.error()) {
        (_, Some(error)) => Alert::new("Database error")
            .destructive()
            .description(error.to_string())
            .into_any_element(),
        (Some(rows), None) if rows.is_empty() => Empty::new()
            .icon(IconName::User)
            .title("No users yet")
            .description("Add one above.")
            .into_any_element(),
        (Some(rows), None) => Table::new()
            .child(
                TableHeader::new().child(
                    TableRow::new()
                        .child(TableHead::new("ID").w(px(60.)).flex_none())
                        .child(TableHead::new("Email"))
                        .child(TableHead::new("Name"))
                        .child(TableHead::new("").w(px(48.)).flex_none()),
                ),
            )
            .child(TableBody::new().children(rows.iter().map(|user| {
                let id = user.id;
                TableRow::new()
                    .child(
                        TableCell::new()
                            .w(px(60.))
                            .flex_none()
                            .child(id.to_string()),
                    )
                    .child(TableCell::new().child(user.email.clone()))
                    // Text from the database may be Arabic: BidiText orders it on every platform.
                    .child(
                        TableCell::new()
                            .child(BidiText::new(user.name.clone().unwrap_or_default())),
                    )
                    .child(
                        TableCell::new().w(px(48.)).flex_none().child(
                            Button::new(("delete-user", id as usize))
                                .ghost()
                                .small()
                                .icon_only(IconName::Trash)
                                .tooltip("Delete")
                                .on_click(move |_, _, cx| {
                                    write_then_refresh(cx, move |db| async move {
                                        User::filter(User::ID.eq(id)).delete(&db).await.map(|_| ())
                                    });
                                }),
                        ),
                    )
            })))
            .into_any_element(),
        (None, None) => div()
            .flex()
            .justify_center()
            .p(px(24.))
            .child(Spinner::new())
            .into_any_element(),
    };

    Card::new()
        .w(px(640.))
        .child(
            CardHeader::new()
                .child(CardTitle::new("Users"))
                .child(CardDescription::new(if users.is_loading() {
                    "Loading…"
                } else {
                    "Stored in PostgreSQL through rok-db."
                })),
        )
        .child(
            CardContent::new()
                .child(
                    Row::new()
                        .spacing(px(8.))
                        .child(
                            Expanded::new().child(Input::new(&email).leading_icon(IconName::Mail)),
                        )
                        .child(Expanded::new().child(Input::new(&name)))
                        .child(
                            Button::new("add-user")
                                .icon(IconName::Plus)
                                .label("Add")
                                .on_click(add),
                        ),
                )
                .child(body),
        )
}

struct UsersApp;

impl Render for UsersApp {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child(
            div()
                .id("users-page")
                .size_full()
                .overflow_y_scroll()
                .p(px(32.))
                .flex()
                .justify_center()
                .child(UsersCard::new()),
        )
    }
}

fn main() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("Set DATABASE_URL, for example postgres://user:password@localhost/app");
        std::process::exit(2);
    };
    Application::new()
        .with_assets(rok_ui::Assets)
        .run(move |cx: &mut App| {
            rok_ui::init(cx);
            // Connect, create the table, then let the queries run.
            let connecting = db::connect(url.clone(), cx);
            cx.spawn(async move |cx| {
                if let Err(error) = connecting.await {
                    eprintln!("could not connect: {error}");
                    return;
                }
                let created = cx
                    .update(|cx| db::run(cx, |db| async move { db.execute(CREATE_TABLE).await }))
                    .ok();
                if let Some(Err(error)) = match created {
                    Some(task) => Some(task.await),
                    None => None,
                } {
                    eprintln!("could not create the table: {error}");
                }
                cx.update(|cx| db::invalidate("users", cx)).ok();
            })
            .detach();

            let bounds = Bounds::centered(None, gpui::size(px(860.), px(640.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("rok-ui + rok-db".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| UsersApp),
            )
            .expect("failed to open the window");
            cx.activate(true);
        });
}
