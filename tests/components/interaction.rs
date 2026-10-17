//! Layers, focus, popovers, reading direction and drawers, driven like a user would.

use gpui::Focusable as _;
use rok_ui::prelude::*;

/// Floating surfaces nested inside each other (a menu in a popover in a dialog).
/// GPUI panics on nested `deferred` elements, so inner layers must become portals.
struct NestedLayersView;

impl Render for NestedLayersView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child(
            Dialog::new("outer-dialog")
                .open(true)
                .title("Dialog")
                .child(
                    Popover::new("inner-popover")
                        .open(true)
                        .trigger(Button::new("popover-trigger").label("Popover"))
                        .child(
                            DropdownMenu::new("innermost-menu")
                                .open(true)
                                .trigger(Button::new("menu-trigger").label("Menu"))
                                .menu(Menu::new().item(MenuItem::new("Item"))),
                        ),
                )
                .child(
                    ContextMenu::new("context-in-dialog")
                        .menu(Menu::new().item(MenuItem::new("Copy")))
                        .child("Area"),
                ),
        )
    }
}

#[gpui::test]
fn nested_floating_layers_render(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    cx.update(|cx| {
        toast(cx, Toast::new("A toast above everything"));
    });
    let (_view, window_context) = cx.add_window_view(|_, _| NestedLayersView);
    window_context.run_until_parked();
    window_context.update(|window, _| window.refresh());
    window_context.run_until_parked();
}

/// Records the laid-out width of a `ButtonGroup` and of its three buttons on their own.
struct ButtonGroupWidthView(std::rc::Rc<std::cell::Cell<(f32, f32)>>);

impl Render for ButtonGroupWidthView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let widths = self.0.clone();
        let probe = move |slot: usize| {
            let widths = widths.clone();
            gpui::canvas(
                move |bounds, _, _| {
                    let (group, row) = widths.get();
                    let width = f32::from(bounds.size.width);
                    widths.set(if slot == 0 {
                        (width, row)
                    } else {
                        (group, width)
                    });
                },
                |_, (), _, _| {},
            )
            .absolute()
            .size_full()
        };
        let buttons = |prefix: &'static str| {
            ["Archive", "Report", "Snooze"]
                .map(|label| Button::new((prefix, label.len())).outline().label(label))
        };
        let [archive, report, snooze] = buttons("group");
        let [plain_archive, plain_report, plain_snooze] = buttons("plain");
        div()
            .w(px(600.))
            .flex()
            .flex_wrap()
            .gap(px(8.))
            .child(
                div()
                    .relative()
                    .child(probe(0))
                    .child(ButtonGroup::new().item(archive).item(report).item(snooze)),
            )
            .child(
                div().relative().child(probe(1)).child(
                    div()
                        .flex()
                        .child(plain_archive)
                        .child(plain_report)
                        .child(plain_snooze),
                ),
            )
    }
}

#[gpui::test]
fn button_group_is_as_wide_as_its_items(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let widths = std::rc::Rc::new(std::cell::Cell::new((0., 0.)));
    let view_widths = widths.clone();
    let (_view, window_context) =
        cx.add_window_view(move |_, _| ButtonGroupWidthView(view_widths.clone()));
    window_context.run_until_parked();
    let (group, row) = widths.get();
    // Joined items share a border, so the group is at most a few pixels narrower.
    assert!(row > 200., "buttons measured {row}px");
    assert!(
        (row - group).abs() <= 3.,
        "group {group}px vs buttons {row}px"
    );
}

/// An input outside a dialog and two inside it.
struct FocusTrapView {
    outside: Entity<InputState>,
    first: Entity<InputState>,
    second: Entity<InputState>,
}

impl Render for FocusTrapView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new().child(Input::new(&self.outside)).child(
            Dialog::new("trap")
                .open(true)
                .title("Edit")
                .child(Input::new(&self.first))
                .child(Input::new(&self.second)),
        )
    }
}

#[gpui::test]
fn dialog_focuses_its_first_field_and_traps_tab(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let (view, window_context) = cx.add_window_view(|_, cx| FocusTrapView {
        outside: cx.new(InputState::new),
        first: cx.new(InputState::new),
        second: cx.new(InputState::new),
    });
    window_context.run_until_parked();
    window_context.update(|window, _| window.refresh());
    window_context.run_until_parked();

    let focused = |window_context: &mut gpui::VisualTestContext,
                   pick: fn(&FocusTrapView) -> &Entity<InputState>| {
        let view = view.clone();
        window_context.update(move |window, cx| {
            let state = pick(view.read(cx)).clone();
            state.read(cx).focus_handle(cx).is_focused(window)
        })
    };

    assert!(
        focused(window_context, |view| &view.first),
        "opening the dialog focuses its first field"
    );
    // Tab cycles through the dialog's own focusable elements and never escapes.
    for _ in 0..8 {
        window_context.simulate_keystrokes("tab");
        window_context.run_until_parked();
        assert!(!focused(window_context, |view| &view.outside));
    }
    for _ in 0..8 {
        window_context.simulate_keystrokes("shift-tab");
        window_context.run_until_parked();
        assert!(!focused(window_context, |view| &view.outside));
    }
    // From the first field, Shift-Tab wraps to the end and Tab wraps back to it.
    window_context.update({
        let view = view.clone();
        move |window, cx| {
            let first = view.read(cx).first.read(cx).focus_handle(cx);
            window.focus(&first);
        }
    });
    window_context.simulate_keystrokes("shift-tab");
    window_context.run_until_parked();
    assert!(!focused(window_context, |view| &view.first));
    window_context.simulate_keystrokes("tab");
    window_context.run_until_parked();
    assert!(
        focused(window_context, |view| &view.first),
        "Tab from the last element wraps to the first"
    );
}

/// A popover whose content holds a second popover. Records each open change as
/// `(popover, open)`.
struct NestedPopoverView(std::rc::Rc<std::cell::RefCell<Vec<(&'static str, bool)>>>);

impl Render for NestedPopoverView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let record = |name: &'static str| {
            let log = self.0.clone();
            move |open: &bool, _: &mut Window, _: &mut App| log.borrow_mut().push((name, *open))
        };
        AppRoot::new().p(px(40.)).child(
            Popover::new("outer")
                .on_open_change(record("outer"))
                .trigger(
                    div()
                        .debug_selector(|| "outer-trigger".into())
                        .child(Button::new("outer-button").label("Outer")),
                )
                .child(
                    Popover::new("inner")
                        .on_open_change(record("inner"))
                        .trigger(
                            div()
                                .debug_selector(|| "inner-trigger".into())
                                .child(Button::new("inner-button").label("Inner")),
                        )
                        .child(
                            div()
                                .debug_selector(|| "inner-body".into())
                                .h(px(60.))
                                .child("Inner content"),
                        ),
                ),
        )
    }
}

#[gpui::test]
fn clicking_a_nested_popover_keeps_its_parent_open(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let view_log = log.clone();
    let (_view, window_context) =
        cx.add_window_view(move |_, _| NestedPopoverView(view_log.clone()));
    window_context.run_until_parked();

    let mut click = |selector: &'static str| {
        let bounds = window_context
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} is not rendered"));
        window_context.simulate_click(bounds.center(), gpui::Modifiers::none());
        window_context.run_until_parked();
    };
    click("outer-trigger");
    click("inner-trigger");
    assert_eq!(*log.borrow(), [("outer", true), ("inner", true)]);

    // Pressing inside the inner popover is outside the outer one, but must not close it.
    click("inner-body");
    assert_eq!(
        *log.borrow(),
        [("outer", true), ("inner", true)],
        "a press inside a nested popover closes nothing"
    );
}

struct FocusVisibleView;

impl Render for FocusVisibleView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        AppRoot::new()
            .child(Button::new("first").label("First"))
            .child(Button::new("second").label("Second"))
    }
}

#[gpui::test]
fn focus_rings_follow_the_last_input_device(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let (_view, window_context) = cx.add_window_view(|_, _| FocusVisibleView);
    window_context.run_until_parked();

    window_context.simulate_keystrokes("tab");
    window_context.run_until_parked();
    assert!(
        rok_ui::sx::focus_visible(),
        "keyboard input shows focus rings"
    );

    window_context.simulate_click(gpui::point(px(10.), px(10.)), gpui::Modifiers::none());
    window_context.run_until_parked();
    assert!(!rok_ui::sx::focus_visible(), "a click hides them");

    rok_ui::sx::set_focus_ring_mode(rok_ui::sx::FocusRingMode::Always);
    assert!(rok_ui::sx::focus_visible(), "Always shows them regardless");
    rok_ui::sx::set_focus_ring_mode(rok_ui::sx::FocusRingMode::KeyboardOnly);
}

/// A row and a grid of three labeled boxes, right to left.
struct RtlLayoutView;

impl Render for RtlLayoutView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let cell = |name: &'static str| div().debug_selector(move || name.into()).size(px(40.));
        AppRoot::new().child(
            Direction::new(TextDirection::Rtl).child(
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .w(px(600.))
                    .child(
                        Row::new()
                            .spacing(px(8.))
                            .child(cell("row-first"))
                            .child(Spacer::new())
                            .child(cell("row-last")),
                    )
                    .child(GridView::count(3).children(["grid-0", "grid-1", "grid-2"].map(cell)))
                    .child(cell("column-start")),
            ),
        )
    }
}

#[gpui::test]
fn layout_widgets_follow_the_reading_direction(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    let (_view, window_context) = cx.add_window_view(|_, _| RtlLayoutView);
    window_context.run_until_parked();
    let mut left = |selector: &'static str| {
        window_context
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} is not rendered"))
            .left()
    };
    // The row starts on the right and the spacer pushes its last child to the left.
    assert!(left("row-first") > left("row-last"));
    assert_eq!(left("row-first") - left("row-last"), px(560.));
    // Grid cells fill from the right.
    assert!(left("grid-0") > left("grid-1"));
    assert!(left("grid-1") > left("grid-2"));
    // A column's cross-axis start is the right edge.
    assert_eq!(left("column-start"), left("row-first"));
}

/// A scaffold whose start drawer is controlled by the view.
struct DrawerShellView {
    drawer_open: bool,
    page: usize,
    log: std::rc::Rc<std::cell::RefCell<Vec<bool>>>,
}

impl Render for DrawerShellView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let log = self.log.clone();
        AppRoot::new().child(
            Scaffold::new("shell")
                .app_bar(AppBar::new().title("Mail"))
                .drawer(
                    NavigationDrawer::new("drawer")
                        .destination(NavigationDestination::new(IconName::Inbox, "Inbox"))
                        .destination(NavigationDestination::new(IconName::Send, "Sent"))
                        .selected_index(self.page)
                        .on_change(cx.listener(|view, index: &usize, _, cx| {
                            view.page = *index;
                            cx.notify();
                        })),
                )
                .drawer_open(self.drawer_open)
                .on_drawer_change(cx.listener(move |view, open: &bool, _, cx| {
                    log.borrow_mut().push(*open);
                    view.drawer_open = *open;
                    cx.notify();
                }))
                .child("Messages"),
        )
    }
}

#[gpui::test]
fn scaffold_drawer_opens_from_the_menu_and_closes_on_navigation(cx: &mut gpui::TestAppContext) {
    cx.update(rok_ui::init);
    // Drawers slide in on real time; without motion they open in one frame.
    rok_ui::motion::set_reduced_motion(true);
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let view_log = log.clone();
    let (view, window_context) = cx.add_window_view(move |_, _| DrawerShellView {
        drawer_open: false,
        page: 0,
        log: view_log.clone(),
    });
    window_context.run_until_parked();

    // The app bar's implied menu button: 8 px padding plus half of a 36 px button.
    window_context.simulate_click(gpui::point(px(26.), px(32.)), gpui::Modifiers::none());
    window_context.run_until_parked();
    assert_eq!(*log.borrow(), [true], "the menu button opens the drawer");

    // Pick the second destination: rows are 48 px under 12 px of padding, 2 px apart.
    window_context.simulate_click(gpui::point(px(100.), px(86.)), gpui::Modifiers::none());
    window_context.run_until_parked();
    assert_eq!(
        *log.borrow(),
        [true, false],
        "picking a destination closes it"
    );
    assert_eq!(view.read_with(window_context, |view, _| view.page), 1);
    rok_ui::motion::set_reduced_motion(false);
}
