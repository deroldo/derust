//! The home screen, built only with typed Stac `v1` values (no JSON strings).
//!
//! Also compiled by the `derust` test-suite (`tests/sdui_stac_v1/example_test.rs`), which checks the
//! result against `fixtures/examples/home_screen.json` (parsed by the real Stac in CI).

use derust::sduix::stacx::v1::prelude::*;

pub fn home_screen() -> Widget {
    Widget::from(
        scaffold().app_bar(app_bar().title(text("Home"))).body(
            column()
                .main_axis_alignment(MainAxisAlignment::Center)
                .spacing(8)
                .children([
                    Widget::from(
                        container()
                            .padding(EdgeInsets::all(16))
                            .color("#EEEEEE")
                            .child(
                                row()
                                    .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                                    .children([
                                        Widget::from(
                                            // state reference: replaced by the Stac client
                                            text(StateRef::new("user.name")).style(
                                                CustomTextStyle::new()
                                                    .font_size(18)
                                                    .font_weight(FontWeight::W700),
                                            ),
                                        ),
                                        Widget::from(
                                            elevated_button()
                                                .on_pressed(
                                                    navigate()
                                                        .route_name("/details")
                                                        .navigation_style(
                                                            NavigationStyle::PushNamed,
                                                        ),
                                                )
                                                .child(text("Details")),
                                        ),
                                    ]),
                            ),
                    ),
                    Widget::from(
                        text_button()
                            .on_pressed(
                                set_value_action().values([StateEntry::new("user.name", "Ada")]),
                            )
                            .child(text("Set name")),
                    ),
                    Widget::from(
                        text_button()
                            .on_pressed(
                                network_request("https://api.example.com/refresh")
                                    .method(Method::Post),
                            )
                            .child(text("Refresh")),
                    ),
                ]),
        ),
    )
}
