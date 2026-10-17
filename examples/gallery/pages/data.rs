//! The data page: tables, lists and charts.

use rok_ui::prelude::*;

use super::{column, demo};

/// A monospace row in the collapsible demo.
#[component]
pub(super) fn RepositoryRow(name: SharedString, cx: &mut App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .px(px(16.))
        .py(px(8.))
        .rounded(theme.radius_medium())
        .border_1()
        .border_color(theme.colors.border)
        .font_family(theme.monospace_font_family.clone())
        .text_sm()
        .child(name)
}

pub(super) fn centered(text: &'static str) -> impl IntoElement {
    div()
        .size_full()
        .flex_dir()
        .items_center()
        .justify_center()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
}

/// Tables and charts.
#[component]
pub fn DataPage() -> impl IntoElement {
    let payments: [(&str, &str, &str); 8] = [
        ("success", "ken99@example.com", "$316.00"),
        ("success", "abe45@example.com", "$242.00"),
        ("processing", "monserrat44@example.com", "$837.00"),
        ("success", "silas22@example.com", "$874.00"),
        ("failed", "carmella@example.com", "$721.00"),
        ("pending", "jason78@example.com", "$1,450.00"),
        ("success", "sarah23@example.com", "$90.00"),
        ("failed", "derek11@example.com", "$612.00"),
    ];
    let months = ["January", "February", "March", "April", "May", "June"];

    let invoices = [
        ("INV001", "Paid", "Credit Card", "$250.00"),
        ("INV002", "Pending", "PayPal", "$150.00"),
        ("INV003", "Unpaid", "Bank Transfer", "$350.00"),
        ("INV004", "Paid", "Credit Card", "$450.00"),
    ];

    div()
        .flex_dir()
        .flex_col()
        .gap(px(16.))
        .child(demo(
            "Data Table",
            "Filter by email, sort by amount, select rows, hide columns, page through results.",
            DataTable::new("payments")
                .column(DataColumn::new("status", "Status").width(px(120.)))
                .column(DataColumn::new("email", "Email").sortable())
                .column(
                    DataColumn::new("amount", "Amount")
                        .sortable()
                        .align_end()
                        .width(px(140.)),
                )
                .rows(payments.iter().map(|(status, email, amount)| {
                    vec![(*status).into(), (*email).into(), (*amount).into()]
                }))
                .filter_column("email", "Filter emails…")
                .selectable(true)
                .page_size(5)
                .row_actions(move |row| {
                    let email = payments[row].1;
                    Menu::new()
                        .label("Actions")
                        .item(MenuItem::new("Copy email").on_select(move |(), _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                email.to_string(),
                            ));
                            toast(cx, Toast::success("Copied to the clipboard"));
                        }))
                        .separator()
                        .item(MenuItem::new("View customer"))
                        .item(MenuItem::new("View payment details"))
                }),
        ))
        .child(
            div()
                .flex_dir()
                .items_start()
                .gap(px(16.))
                .child(
                    column().child(demo(
                        "Bar Chart",
                        "January – June 2024",
                        Chart::new("visitors-bar", ChartKind::Bar)
                            .categories(months)
                            .series(ChartSeries::new(
                                "Desktop",
                                [186., 305., 237., 73., 209., 214.],
                            ))
                            .series(ChartSeries::new(
                                "Mobile",
                                [80., 200., 120., 190., 130., 140.],
                            ))
                            .legend(true),
                    )),
                )
                .child(
                    column().child(demo(
                        "Area Chart",
                        "Showing total visitors for the last 6 months",
                        Chart::new("visitors-area", ChartKind::Area)
                            .categories(months)
                            .series(ChartSeries::new(
                                "Desktop",
                                [186., 305., 237., 73., 209., 214.],
                            ))
                            .series(ChartSeries::new(
                                "Mobile",
                                [80., 200., 120., 190., 130., 140.],
                            ))
                            .legend(true),
                    )),
                ),
        )
        .child(
            div()
                .flex_dir()
                .items_start()
                .gap(px(16.))
                .child(
                    column().child(demo(
                        "Line Chart",
                        "With a y-axis.",
                        Chart::new("visitors-line", ChartKind::Line)
                            .categories(months)
                            .series(ChartSeries::new(
                                "Desktop",
                                [186., 305., 237., 73., 209., 214.],
                            ))
                            .y_axis(true),
                    )),
                )
                .child(
                    column().child(demo(
                        "Donut Chart",
                        "Browser share",
                        Chart::new("browsers", ChartKind::Donut)
                            .categories(["Chrome", "Safari", "Firefox", "Edge", "Other"])
                            .series(ChartSeries::new("Visitors", [275., 200., 187., 173., 90.]))
                            .center_label("925", "Visitors")
                            .legend(true),
                    )),
                ),
        )
        .child(demo(
            "Table",
            "A list of your recent invoices.",
            Table::new()
                .child(
                    TableHeader::new().child(
                        TableRow::new()
                            .child(TableHead::new("Invoice").w(px(120.)).flex_none())
                            .child(TableHead::new("Status"))
                            .child(TableHead::new("Method"))
                            .child(TableHead::new("Amount").justify_end()),
                    ),
                )
                .child(TableBody::new().children(invoices.iter().map(
                    |(invoice, status, method, amount)| {
                        TableRow::new()
                            .child(
                                TableCell::new()
                                    .w(px(120.))
                                    .flex_none()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(*invoice),
                            )
                            .child(TableCell::new().child(*status))
                            .child(TableCell::new().child(*method))
                            .child(TableCell::new().flex_dir().justify_end().child(*amount))
                    },
                )))
                .child(
                    TableFooter::new().child(
                        TableRow::new()
                            .border_b_0()
                            .child(TableCell::new().child("Total"))
                            .child(TableCell::new().flex_dir().justify_end().child("$1,200.00")),
                    ),
                )
                .child(TableCaption::new("A list of your recent invoices.")),
        ))
}
