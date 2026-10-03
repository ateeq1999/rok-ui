//! rok-db integration against a real `PostgreSQL` server. Skipped unless
//! `ROK_UI_TEST_DATABASE_URL` is set, for example:
//!
//! ```sh
//! docker run -d --rm -e POSTGRES_PASSWORD=rok -p 55432:5432 postgres:17-alpine
//! ROK_UI_TEST_DATABASE_URL=postgres://postgres:rok@127.0.0.1:55432/postgres \
//!     cargo test --features db --test db
//! ```

#![cfg(feature = "db")]

use std::time::{Duration, Instant};

use rok_ui::{
    db::{self, rok_db::prelude::*, DbError},
    query, query_key,
};

#[derive(Debug, Clone, Model)]
#[rok(crate = "rok_ui::db::rok_db", table = "rok_ui_watched_notes")]
struct Note {
    #[rok(primary_key)]
    id: i64,
    title: String,
}

fn notes_query() -> query::QueryOptions<Vec<Note>> {
    db::db_query(query_key!["rok_ui_watched_notes"], |db| async move {
        Note::query().all(&db).await
    })
}

fn is_invalidated(cx: &mut gpui::TestAppContext) -> bool {
    cx.update(|cx| query::queries(cx))
        .iter()
        .any(|info| info.key == query_key!["rok_ui_watched_notes"] && info.is_invalidated)
}

#[gpui::test]
fn the_app_connects_and_runs_database_work(cx: &mut gpui::TestAppContext) {
    let Ok(url) = std::env::var("ROK_UI_TEST_DATABASE_URL") else {
        eprintln!("ROK_UI_TEST_DATABASE_URL is not set; skipping");
        return;
    };
    // The database answers on real threads, outside the test scheduler.
    cx.executor().allow_parking();

    let before = cx.update(|cx| db::run(cx, |db| async move { db.ping().await }));
    assert_eq!(cx.executor().block_test(before), Err(DbError::NotConnected));

    let connecting = cx.update(|cx| db::connect(url, cx));
    cx.executor()
        .block_test(connecting)
        .expect("connects to the test database");
    assert!(cx.update(|cx| db::connection(cx).is_some()));

    let ping = cx.update(|cx| db::run(cx, |db| async move { db.ping().await }));
    assert_eq!(cx.executor().block_test(ping), Ok(()));

    let broken = cx.update(|cx| {
        db::run(cx, |db| async move {
            db.execute("SELECT * FROM no_such_table").await
        })
    });
    match cx.executor().block_test(broken) {
        Err(DbError::Failed(message)) => assert!(message.contains("no_such_table"), "{message}"),
        other => panic!("expected a database error, got {other:?}"),
    }
}

#[gpui::test]
fn watched_tables_invalidate_their_queries(cx: &mut gpui::TestAppContext) {
    let Ok(url) = std::env::var("ROK_UI_TEST_DATABASE_URL") else {
        eprintln!("ROK_UI_TEST_DATABASE_URL is not set; skipping");
        return;
    };
    cx.executor().allow_parking();

    let unconnected = cx.update(db::watch_changes::<Note>);
    assert_eq!(
        cx.executor().block_test(unconnected),
        Err(DbError::NotConnected)
    );

    let connecting = cx.update(|cx| db::connect(url.clone(), cx));
    cx.executor().block_test(connecting).expect("connects");
    let setup = cx.update(|cx| {
        db::run(cx, |db| async move {
            db.execute("DROP TABLE IF EXISTS rok_ui_watched_notes")
                .await?;
            db.execute(
                "CREATE TABLE rok_ui_watched_notes (id BIGSERIAL PRIMARY KEY, title TEXT NOT NULL)",
            )
            .await?;
            Note::install_change_notifications(&db).await
        })
    });
    cx.executor().block_test(setup).expect("creates the table");

    let fetch = cx.update(|cx| query::fetch_query(cx, &notes_query()));
    assert!(cx.executor().block_test(fetch).expect("reads").is_empty());
    assert!(!is_invalidated(cx));

    let watching = cx.update(db::watch_changes::<Note>);

    // Another connection writes, as another process would. The listener subscribes in the
    // background, so write until a change arrives.
    let writer = cx
        .executor()
        .block_test(async {
            db::runtime()
                .spawn(async move { Db::connect(&url).await })
                .await
                .expect("the task finishes")
        })
        .expect("a second connection");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !is_invalidated(cx) {
        assert!(Instant::now() < deadline, "no change notification arrived");
        let db = writer.clone();
        db::runtime()
            .block_on(async move {
                db.execute("INSERT INTO rok_ui_watched_notes (title) VALUES ('hello')")
                    .await
            })
            .expect("inserts");
        std::thread::sleep(Duration::from_millis(100));
        cx.run_until_parked();
    }

    drop(watching);
    let cleanup = cx.update(|cx| {
        db::run(cx, |db| async move {
            db.execute("DROP TABLE rok_ui_watched_notes").await
        })
    });
    cx.executor().block_test(cleanup).expect("drops the table");
}
