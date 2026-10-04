//! Navigation-scoped memoization. Its own test binary: memoized results are process-wide, so
//! navigations in other tests running in parallel would clear them mid-test.

use gpui::TestAppContext;
use rok_ui::router;

static LOOKUPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[rok_ui::memoize(scope = navigation)]
async fn page_lookup(page: u32) -> u32 {
    LOOKUPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    page
}

#[gpui::test]
fn navigating_forgets_navigation_scoped_memos(cx: &mut TestAppContext) {
    use std::sync::atomic::Ordering;

    cx.update(rok_ui::init);
    let lookup = || rok_ui::runtime::block_on(page_lookup(1));
    lookup();
    lookup();
    assert_eq!(LOOKUPS.load(Ordering::SeqCst), 1);
    cx.update(|cx| router::navigate("/elsewhere", cx));
    lookup();
    assert_eq!(LOOKUPS.load(Ordering::SeqCst), 2);
}
