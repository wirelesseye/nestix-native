use nestix_native_core::*;

#[test]
fn style_macro_builds_class_rule() {
    let sheet = style! {
        .counter.selected {
            bg_color: #FFFFFF;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("counter selected")));

    assert_eq!(props.bg_color, Some(Color::WHITE));
}

#[test]
fn style_macro_supports_multiple_rules_and_selectors() {
    let sheet = style! {
        .counter, .button.primary {
            bg-color: #FFFFFF;
        }

        .counter.selected {
            --text-color: red;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("button primary")));
    assert_eq!(props.bg_color, Some(Color::WHITE));

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("counter selected")));
    assert_eq!(props.custom("--text-color"), Some("red"));
}

#[test]
fn style_macro_supports_child_selectors() {
    let sheet = style! {
        .panel > .button {
            bg_color: red;
        }
    };

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("button")).with_ancestors([ClassList::from("panel")]),
    );
    assert_eq!(props.bg_color, Some(Color::RED));

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("button"))
            .with_ancestors([ClassList::from("section"), ClassList::from("panel")]),
    );
    assert_eq!(props.bg_color, None);
}

#[test]
fn style_macro_supports_descendant_selectors() {
    let sheet = style! {
        .panel >> .button {
            bg_color: blue;
        }
    };

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("button"))
            .with_ancestors([ClassList::from("section"), ClassList::from("panel")]),
    );

    assert_eq!(props.bg_color, Some(Color::BLUE));
}

#[test]
fn combinator_specificity_competes_with_plain_selectors() {
    let sheet = style! {
        .button.primary {
            bg_color: red;
        }

        .panel > .button {
            bg_color: blue;
        }
    };

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("button primary"))
            .with_ancestors([ClassList::from("panel")]),
    );

    assert_eq!(props.bg_color, Some(Color::BLUE));
}

#[test]
fn style_resolution_prefers_specificity_before_source_order() {
    let sheet = style! {
        .counter.selected {
            bg_color: red;
        }

        .counter {
            bg_color: blue;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("counter selected")));

    assert_eq!(props.bg_color, Some(Color::RED));
}

#[test]
fn style_sheets_merge_with_later_sheet_as_override() {
    let base = style! {
        .counter {
            bg_color: red;
        }
    };
    let local = style! {
        .counter {
            bg_color: blue;
        }
    };

    let props = base
        .merged(&local)
        .matched_props(&MatchContext::new(ClassList::from("counter")));

    assert_eq!(props.bg_color, Some(Color::BLUE));
}

#[test]
fn style_macro_embeds_style_sheets_in_source_order() {
    let embedded = style! {
        .counter {
            bg_color: blue;
            width: 240px;
        }
    };

    let sheet = style! {
        .counter {
            bg_color: red;
        }

        $(embedded)

        .counter {
            width: 320px;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("counter")));

    assert_eq!(props.bg_color, Some(Color::BLUE));
    assert_eq!(props.width, Some(Dimension::from(320.0)));
}

#[test]
fn style_macro_with_inserted_value_builds_style_sheet() {
    let bg_color = nestix::create_state(Color::WHITE);
    let sheet = style! {
        .counter {
            bg_color: $(bg_color.get());
            --label: $(format!("count-{}", 1));
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("counter")));
    assert_eq!(props.bg_color, Some(Color::WHITE));
    assert_eq!(props.custom("--label"), Some("count-1"));

    bg_color.set(Color::BLACK);

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("counter")));
    assert_eq!(props.bg_color, Some(Color::WHITE));
}

#[test]
fn style_macro_can_be_wrapped_in_computed_for_dynamic_style_sheets() {
    let bg_color = nestix::create_state(Color::WHITE);
    let sheet = nestix::computed!(
        [bg_color]
            || style! {
                .counter {
                    bg_color: $(bg_color.get());
                }
            }
    );

    let props = sheet
        .get()
        .matched_props(&MatchContext::new(ClassList::from("counter")));
    assert_eq!(props.bg_color, Some(Color::WHITE));

    bg_color.set(Color::BLACK);

    let props = sheet
        .get()
        .matched_props(&MatchContext::new(ClassList::from("counter")));
    assert_eq!(props.bg_color, Some(Color::BLACK));
}

#[test]
fn style_macro_supports_view_props() {
    let sheet = style! {
        .panel {
            left: 1px;
            top: 2px;
            width: 320px;
            height: auto;
            margin-left: 3px;
            margin-right: 4px;
            margin-top: 5px;
            margin-bottom: 6px;
            grow: 2;
            align-self: center;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("panel")));

    assert_eq!(props.left, Some(Dimension::from(1.0)));
    assert_eq!(props.top, Some(Dimension::from(2.0)));
    assert_eq!(props.width, Some(Dimension::from(320.0)));
    assert_eq!(props.height, Some(Dimension::Auto));
    assert_eq!(props.margin_left, Some(Dimension::from(3.0)));
    assert_eq!(props.margin_right, Some(Dimension::from(4.0)));
    assert_eq!(props.margin_top, Some(Dimension::from(5.0)));
    assert_eq!(props.margin_bottom, Some(Dimension::from(6.0)));
    assert_eq!(props.grow, Some(2.0));
    assert_eq!(props.align_self, Some(AlignItems::Center));
}

#[test]
fn style_macro_supports_flex_view_props() {
    let sheet = style! {
        .panel {
            flex-direction: row-reverse;
            align-items: stretch;
            flex-wrap: wrap;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("panel")));

    assert_eq!(props.flex_direction, Some(FlexDirection::RowReverse));
    assert_eq!(props.align_items, Some(AlignItems::Stretch));
    assert_eq!(props.flex_wrap, Some(FlexWrap::Wrap));
}

#[test]
fn style_margin_shorthand_expands_and_cascades_per_edge() {
    let sheet = style! {
        .panel {
            margin: 8px;
            margin-left: 16px;
        }

        .panel.selected {
            margin-right: 24px;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("panel selected")));

    assert_eq!(props.margin_top, Some(Dimension::from(8.0)));
    assert_eq!(props.margin_bottom, Some(Dimension::from(8.0)));
    assert_eq!(props.margin_left, Some(Dimension::from(16.0)));
    assert_eq!(props.margin_right, Some(Dimension::from(24.0)));
}

#[test]
fn class_list_can_include_renderer_default_classes() {
    let class_list = ClassList::from("primary").with_defaults(&["__Button", "__appkit_Button"]);

    assert!(class_list.contains("primary"));
    assert!(class_list.contains("__Button"));
    assert!(class_list.contains("__appkit_Button"));
}
