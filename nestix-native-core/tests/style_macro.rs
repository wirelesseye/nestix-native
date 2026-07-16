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
fn style_macro_supports_gap() {
    let sheet = style! {
        .stack {
            gap: 12px;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("stack")));

    assert_eq!(props.gap, Some(Dimension::from(12.0)));
}

#[test]
fn style_macro_supports_appearance() {
    let inserted = Appearance::None;
    let sheet = style! {
        .native {
            appearance: native;
        }
        .automatic {
            appearance: auto;
        }
        .custom {
            appearance: $(inserted);
        }
    };

    assert_eq!(
        sheet
            .matched_props(&MatchContext::new(ClassList::from("native")))
            .appearance,
        Some(Appearance::Native)
    );
    assert_eq!(
        sheet
            .matched_props(&MatchContext::new(ClassList::from("automatic")))
            .appearance,
        Some(Appearance::Auto)
    );
    assert_eq!(
        sheet
            .matched_props(&MatchContext::new(ClassList::from("custom")))
            .appearance,
        Some(Appearance::None)
    );
}

#[test]
fn style_macro_supports_font_props() {
    let sheet = style! {
        .label {
            font_family: "Helvetica Neue";
            font_size: 16px;
            font_weight: semi-bold;
            font_style: italic;
            text_color: #123456;
        }
        .body {
            font_family: Avenir;
        }
        .numeric_weight {
            font_weight: 345;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("label")));
    assert_eq!(props.font_family.as_deref(), Some("Helvetica Neue"));
    assert_eq!(props.font_size, Some(16.0));
    assert_eq!(props.font_weight, Some(FontWeight::SemiBold));
    assert_eq!(props.font_style, Some(FontStyle::Italic));
    assert_eq!(
        props.text_color,
        Some(Color::RGB(RGBColor::from_rgb(0x12, 0x34, 0x56)))
    );
    let body = sheet.matched_props(&MatchContext::new(ClassList::from("body")));
    assert_eq!(body.font_family.as_deref(), Some("Avenir"));
    let numeric = sheet.matched_props(&MatchContext::new(ClassList::from("numeric_weight")));
    assert_eq!(numeric.font_weight, Some(FontWeight::Numeric(345)));
}

#[test]
fn style_macro_supports_inserted_font_props() {
    let family = "Avenir Next".to_string();
    let size = 18.0;
    let weight = FontWeight::Bold;
    let style = FontStyle::Normal;
    let color = Color::BLUE;
    let sheet = style! {
        .label {
            font_family: $(family.clone());
            font_size: $(size);
            font_weight: $(weight);
            font_style: $(style);
            text_color: $(color);
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("label")));
    assert_eq!(props.font_family, Some(family));
    assert_eq!(props.font_size, Some(size));
    assert_eq!(props.font_weight, Some(weight));
    assert_eq!(props.font_style, Some(style));
    assert_eq!(props.text_color, Some(color));
}

#[test]
fn global_values_resolve_against_parent_style() {
    let mut parent = ResolvedStyle::default();
    parent.bg_color = Some(Color::RED);
    parent.font_size = Some(18.0);
    parent.width = Some(Dimension::from(240.0));
    parent.margin_left = Some(Dimension::from(12.0));
    parent.margin_right = Some(Dimension::from(12.0));
    parent.margin_top = Some(Dimension::from(12.0));
    parent.margin_bottom = Some(Dimension::from(12.0));
    let sheet = style! {
        .child {
            bg_color: initial;
            font_size: unset;
            width: inherit;
            margin: inherit;
            margin_left: initial;
        }
    };

    let child = sheet
        .matched_props_with_parent(&MatchContext::new(ClassList::from("child")), Some(&parent));

    assert_eq!(child.bg_color, None);
    assert_eq!(child.font_size, Some(18.0));
    assert_eq!(child.width, Some(Dimension::from(240.0)));
    assert_eq!(child.margin_left, None);
    assert_eq!(child.margin_right, Some(Dimension::from(12.0)));
    assert_eq!(child.margin_top, Some(Dimension::from(12.0)));
    assert_eq!(child.margin_bottom, Some(Dimension::from(12.0)));
}

#[test]
fn every_builtin_property_accepts_a_global_value() {
    let sheet = style! {
        .all {
            appearance: initial;
            bg_color: initial;
            font_family: initial;
            font_size: initial;
            font_weight: initial;
            font_style: initial;
            text_color: initial;
            left: initial;
            top: initial;
            width: initial;
            height: initial;
            margin: initial;
            margin_horizontal: initial;
            margin_vertical: initial;
            margin_left: initial;
            margin_right: initial;
            margin_top: initial;
            margin_bottom: initial;
            padding: initial;
            padding_horizontal: initial;
            padding_vertical: initial;
            padding_left: initial;
            padding_right: initial;
            padding_top: initial;
            padding_bottom: initial;
            flex_grow: initial;
            flex_basis: initial;
            flex_shrink: initial;
            align_self: initial;
            flex_direction: initial;
            align_items: initial;
            justify_content: initial;
            flex_wrap: initial;
            gap: initial;
        }
    };

    assert_eq!(
        sheet.matched_props(&MatchContext::new(ClassList::from("all"))),
        ResolvedStyle::default()
    );
}

#[test]
fn initial_blocks_natural_inheritance_and_unset_resets_non_inherited_props() {
    let mut parent = ResolvedStyle::default();
    parent.font_family = Some("Avenir".to_string());
    parent.text_color = Some(Color::BLUE);
    parent.width = Some(Dimension::from(300.0));
    let sheet = style! {
        .initial_font {
            font_family: initial;
        }
        .unset_width {
            width: unset;
        }
        .ordinary_child {
            height: auto;
        }
    };

    let initial_font = sheet.matched_props_with_parent(
        &MatchContext::new(ClassList::from("initial_font")),
        Some(&parent),
    );
    assert_eq!(initial_font.font_family, None);
    assert_eq!(initial_font.text_color, Some(Color::BLUE));

    let unset_width = sheet.matched_props_with_parent(
        &MatchContext::new(ClassList::from("unset_width")),
        Some(&parent),
    );
    assert_eq!(unset_width.width, None);
    assert_eq!(unset_width.font_family.as_deref(), Some("Avenir"));

    let ordinary = sheet.matched_props_with_parent(
        &MatchContext::new(ClassList::from("ordinary_child")),
        Some(&parent),
    );
    assert_eq!(ordinary.font_family.as_deref(), Some("Avenir"));
    assert_eq!(ordinary.width, None);
}

#[test]
fn inherit_without_parent_uses_initial_value() {
    let sheet = style! {
        .root {
            font_weight: inherit;
            width: inherit;
        }
    };

    let root = sheet.matched_props(&MatchContext::new(ClassList::from("root")));

    assert_eq!(root.font_weight, None);
    assert_eq!(root.width, None);
}

#[test]
fn global_values_participate_in_specificity_and_source_order() {
    let mut parent = ResolvedStyle::default();
    parent.width = Some(Dimension::from(180.0));
    let sheet = style! {
        .panel.selected {
            width: inherit;
        }
        .panel {
            width: initial;
            width: 90px;
        }
    };

    let selected = sheet.matched_props_with_parent(
        &MatchContext::new(ClassList::from("panel selected")),
        Some(&parent),
    );
    assert_eq!(selected.width, Some(Dimension::from(180.0)));

    let panel = sheet
        .matched_props_with_parent(&MatchContext::new(ClassList::from("panel")), Some(&parent));
    assert_eq!(panel.width, Some(Dimension::from(90.0)));
}

#[test]
fn custom_properties_keep_global_words_as_strings() {
    let sheet = style! {
        .panel {
            --inherit_token: inherit;
            --initial_token: initial;
            --unset_token: unset;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("panel")));
    assert_eq!(props.custom("--inherit_token"), Some("inherit"));
    assert_eq!(props.custom("--initial_token"), Some("initial"));
    assert_eq!(props.custom("--unset_token"), Some("unset"));
}

#[test]
fn inherited_style_uses_parent_inline_view_value() {
    let matched = nestix::computed!(
        [] || {
            let mut style = ResolvedStyle::default();
            style.width = Some(Dimension::from(100.0));
            Some(style)
        }
    );
    let view = nestix::build_props!(ViewProps(.width = Dimension::from(260.0)));
    let parent = resolved_view_style(matched, &view).get().unwrap();
    let sheet = style! {
        .child {
            width: inherit;
        }
    };

    let child = sheet
        .matched_props_with_parent(&MatchContext::new(ClassList::from("child")), Some(&parent));

    assert_eq!(parent.width, Some(Dimension::from(260.0)));
    assert_eq!(child.width, Some(Dimension::from(260.0)));
}

#[test]
fn text_and_button_accept_nested_font_props() {
    let text = nestix::build_props!(TextProps(
        "Hello",
        .font(.font_size = Some(18.0), .font_weight = Some(FontWeight::Bold))
    ));
    let button = nestix::build_props!(ButtonProps(
        .title = "Save",
        .font(.font_family = Some("Avenir".to_string()), .text_color = Some(Color::RED))
    ));

    assert_eq!(text.font.font_size.get(), Some(18.0));
    assert_eq!(text.font.font_weight.get(), Some(FontWeight::Bold));
    assert_eq!(button.font.font_family.get().as_deref(), Some("Avenir"));
    assert_eq!(button.font.text_color.get(), Some(Color::RED));
}

#[test]
fn button_defaults_to_native_appearance_and_auto_padding() {
    let button = nestix::build_props!(ButtonProps());
    let disabled_button = nestix::build_props!(ButtonProps(.disabled = true));
    let padding = button.container.padding().get();
    let container_padding = ContainerProps::default().padding().get();

    assert_eq!(button.appearance.get(), Appearance::Native);
    assert!(!button.disabled.get());
    assert!(disabled_button.disabled.get());
    assert_eq!(padding.top, Dimension::Auto);
    assert_eq!(padding.right, Dimension::Auto);
    assert_eq!(padding.bottom, Dimension::Auto);
    assert_eq!(padding.left, Dimension::Auto);
    assert_eq!(container_padding.top, Dimension::from(0));
    assert_eq!(container_padding.right, Dimension::from(0));
    assert_eq!(container_padding.bottom, Dimension::from(0));
    assert_eq!(container_padding.left, Dimension::from(0));
}

#[test]
fn button_accepts_nested_padding_and_styles_override_auto_edges() {
    let button = nestix::build_props!(ButtonProps(
        .appearance = Appearance::Auto,
        .container(.padding_horizontal = Dimension::from(12))
    ));
    let inline = button.container.padding().get();
    let default_button = nestix::build_props!(ButtonProps());
    let mut style = ResolvedStyle::default();
    style.padding_top = Some(Dimension::from(8));
    let resolved = style_padding_with_default(
        Some(&style),
        default_button.container.padding().get(),
        Dimension::Auto,
    );

    assert_eq!(button.appearance.get(), Appearance::Auto);
    assert_eq!(inline.left, Dimension::from(12));
    assert_eq!(inline.right, Dimension::from(12));
    assert_eq!(resolved.top, Dimension::from(8));
    assert_eq!(resolved.bottom, Dimension::Auto);
}

#[test]
fn appearance_uses_inline_or_stylesheet_precedence() {
    let mut style = ResolvedStyle::default();
    style.appearance = Some(Appearance::Auto);

    assert_eq!(
        style_appearance(Some(&style), Appearance::Native),
        Appearance::Auto
    );
    assert_eq!(
        style_appearance(Some(&style), Appearance::None),
        Appearance::None
    );
    assert_eq!(
        style_appearance(None, Appearance::Native),
        Appearance::Native
    );
}

#[test]
fn style_macro_supports_multiple_rules_and_selectors() {
    let sheet = style! {
        .counter, .button.primary {
            bg_color: #FFFFFF;
        }

        .counter.selected {
            --text_color: red;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("button primary")));
    assert_eq!(props.bg_color, Some(Color::WHITE));

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("counter selected")));
    assert_eq!(props.custom("--text_color"), Some("red"));
}

#[test]
fn style_macro_supports_not_selectors() {
    let sheet = style! {
        .button:not(.disabled) {
            bg_color: red;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("button")));
    assert_eq!(props.bg_color, Some(Color::RED));

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("button disabled")));
    assert_eq!(props.bg_color, None);
}

#[test]
fn style_macro_supports_first_and_last_child_selectors() {
    let sheet = style! {
        .item:first-child {
            --first: yes;
        }
        .item:last-child {
            --last: yes;
        }
    };

    let first =
        sheet.matched_props(&MatchContext::new(ClassList::from("item")).with_child_position(1, 3));
    assert_eq!(first.custom("--first"), Some("yes"));
    assert_eq!(first.custom("--last"), None);

    let middle =
        sheet.matched_props(&MatchContext::new(ClassList::from("item")).with_child_position(2, 3));
    assert_eq!(middle.custom("--first"), None);
    assert_eq!(middle.custom("--last"), None);

    let last =
        sheet.matched_props(&MatchContext::new(ClassList::from("item")).with_child_position(3, 3));
    assert_eq!(last.custom("--first"), None);
    assert_eq!(last.custom("--last"), Some("yes"));

    let unparented = sheet.matched_props(&MatchContext::new(ClassList::from("item")));
    assert_eq!(unparented.custom("--first"), None);
    assert_eq!(unparented.custom("--last"), None);
}

#[test]
fn style_macro_supports_full_nth_child_formulas() {
    let sheet = style! {
        .item:nth-child(2) { --exact: yes; }
        .item:nth-child(odd) { --odd: yes; }
        .item:nth-child(even) { --even: yes; }
        .item:nth-child(3n) { --multiple: yes; }
        .item:nth-child(2n + 1) { --spaced: yes; }
        .item:nth-child(-n + 3) { --first_three: yes; }
        .item:nth-child(0) { --zero: yes; }
    };

    let second =
        sheet.matched_props(&MatchContext::new(ClassList::from("item")).with_child_position(2, 8));
    assert_eq!(second.custom("--exact"), Some("yes"));
    assert_eq!(second.custom("--odd"), None);
    assert_eq!(second.custom("--even"), Some("yes"));
    assert_eq!(second.custom("--first_three"), Some("yes"));
    assert_eq!(second.custom("--zero"), None);

    let third =
        sheet.matched_props(&MatchContext::new(ClassList::from("item")).with_child_position(3, 8));
    assert_eq!(third.custom("--odd"), Some("yes"));
    assert_eq!(third.custom("--multiple"), Some("yes"));
    assert_eq!(third.custom("--spaced"), Some("yes"));
    assert_eq!(third.custom("--first_three"), Some("yes"));

    let fifth =
        sheet.matched_props(&MatchContext::new(ClassList::from("item")).with_child_position(5, 8));
    assert_eq!(fifth.custom("--spaced"), Some("yes"));
    assert_eq!(fifth.custom("--first_three"), None);
}

#[test]
fn structural_pseudo_classes_work_inside_combinators() {
    let sheet = style! {
        .panel:first-child > .button {
            --parent_first: yes;
        }
        .item:first-child + .item {
            --after_first: yes;
        }
    };

    let child = sheet.matched_props(
        &MatchContext::new(ClassList::from("button"))
            .with_ancestors([ClassList::from("panel")])
            .with_ancestor_positions([Some((1, 2))]),
    );
    assert_eq!(child.custom("--parent_first"), Some("yes"));

    let sibling = sheet.matched_props(
        &MatchContext::new(ClassList::from("item"))
            .with_previous_siblings([ClassList::from("item")])
            .with_child_position(2, 3),
    );
    assert_eq!(sibling.custom("--after_first"), Some("yes"));
}

#[test]
fn structural_pseudo_classes_contribute_class_specificity() {
    let sheet = style! {
        .item:first-child {
            bg_color: red;
        }
        .item {
            bg_color: blue;
        }
    };

    let props =
        sheet.matched_props(&MatchContext::new(ClassList::from("item")).with_child_position(1, 2));
    assert_eq!(props.bg_color, Some(Color::RED));
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
fn style_macro_supports_adjacent_sibling_selectors() {
    let sheet = style! {
        .label + .input {
            bg_color: red;
        }
    };

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("input"))
            .with_previous_siblings([ClassList::from("label")]),
    );
    assert_eq!(props.bg_color, Some(Color::RED));

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("input"))
            .with_previous_siblings([ClassList::from("spacer"), ClassList::from("label")]),
    );
    assert_eq!(props.bg_color, None);
}

#[test]
fn style_macro_supports_subsequent_sibling_selectors() {
    let sheet = style! {
        .label ~ .input {
            bg_color: blue;
        }
    };

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("input"))
            .with_previous_siblings([ClassList::from("spacer"), ClassList::from("label")]),
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
fn not_selector_specificity_competes_with_plain_selectors() {
    let sheet = style! {
        .button:not(.disabled) {
            bg_color: red;
        }

        .button {
            bg_color: blue;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("button")));

    assert_eq!(props.bg_color, Some(Color::RED));
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
            margin_left: 3px;
            margin_right: 4px;
            margin_top: 5px;
            margin_bottom: 6px;
            flex_grow: 2;
            flex_basis: 25px;
            flex_shrink: 3;
            align_self: center;
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
    assert_eq!(props.flex_grow, Some(2.0));
    assert_eq!(props.flex_basis, Some(Dimension::from(25.0)));
    assert_eq!(props.flex_shrink, Some(3.0));
    assert_eq!(props.align_self, Some(AlignItems::Center));
}

#[test]
fn style_macro_supports_flex_view_props() {
    let sheet = style! {
        .panel {
            flex_direction: row-reverse;
            align_items: stretch;
            justify_content: space-between;
            flex_wrap: wrap;
            padding_left: 3px;
            padding_right: 4px;
            padding_top: 5px;
            padding_bottom: 6px;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("panel")));

    assert_eq!(props.flex_direction, Some(FlexDirection::RowReverse));
    assert_eq!(props.align_items, Some(AlignItems::Stretch));
    assert_eq!(props.justify_content, Some(JustifyContent::SpaceBetween));
    assert_eq!(props.flex_wrap, Some(FlexWrap::Wrap));
    assert_eq!(props.padding_left, Some(Dimension::from(3.0)));
    assert_eq!(props.padding_right, Some(Dimension::from(4.0)));
    assert_eq!(props.padding_top, Some(Dimension::from(5.0)));
    assert_eq!(props.padding_bottom, Some(Dimension::from(6.0)));
}

#[test]
fn style_margin_shorthand_expands_and_cascades_per_edge() {
    let sheet = style! {
        .panel {
            margin: 8px;
            margin_left: 16px;
        }

        .panel.selected {
            margin_right: 24px;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("panel selected")));

    assert_eq!(props.margin_top, Some(Dimension::from(8.0)));
    assert_eq!(props.margin_bottom, Some(Dimension::from(8.0)));
    assert_eq!(props.margin_left, Some(Dimension::from(16.0)));
    assert_eq!(props.margin_right, Some(Dimension::from(24.0)));
}

#[test]
fn style_margin_shorthand_evaluates_inserted_value_once() {
    let mut calls = 0;
    let sheet = style! {
        .panel {
            margin: $({
                calls += 1;
                Dimension::from(8.0)
            });
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("panel")));

    assert_eq!(calls, 1);
    assert_eq!(props.margin_top, Some(Dimension::from(8.0)));
    assert_eq!(props.margin_right, Some(Dimension::from(8.0)));
    assert_eq!(props.margin_bottom, Some(Dimension::from(8.0)));
    assert_eq!(props.margin_left, Some(Dimension::from(8.0)));
}

#[test]
fn style_padding_shorthand_expands_and_cascades_per_edge() {
    let sheet = style! {
        .panel {
            padding: 8px;
            padding_left: 16px;
        }

        .panel.selected {
            padding_right: 24px;
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("panel selected")));

    assert_eq!(props.padding_top, Some(Dimension::from(8.0)));
    assert_eq!(props.padding_bottom, Some(Dimension::from(8.0)));
    assert_eq!(props.padding_left, Some(Dimension::from(16.0)));
    assert_eq!(props.padding_right, Some(Dimension::from(24.0)));
}

#[test]
fn style_padding_shorthand_evaluates_inserted_value_once() {
    let mut calls = 0;
    let sheet = style! {
        .panel {
            padding: $({
                calls += 1;
                Dimension::from(8.0)
            });
        }
    };

    let props = sheet.matched_props(&MatchContext::new(ClassList::from("panel")));

    assert_eq!(calls, 1);
    assert_eq!(props.padding_top, Some(Dimension::from(8.0)));
    assert_eq!(props.padding_right, Some(Dimension::from(8.0)));
    assert_eq!(props.padding_bottom, Some(Dimension::from(8.0)));
    assert_eq!(props.padding_left, Some(Dimension::from(8.0)));
}

#[test]
fn class_list_can_include_renderer_default_classes() {
    let class_list = ClassList::from("primary").with_defaults(&["__Button", "__appkit_Button"]);

    assert!(class_list.contains("primary"));
    assert!(class_list.contains("__Button"));
    assert!(class_list.contains("__appkit_Button"));
}

#[test]
fn style_macro_supports_recursive_implicit_nesting() {
    let sheet = style! {
        .panel {
            .section {
                .button {
                    bg_color: red;
                }
            }
        }
    };

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("button"))
            .with_ancestors([ClassList::from("section"), ClassList::from("panel")]),
    );
    assert_eq!(props.bg_color, Some(Color::RED));

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("button")).with_ancestors([ClassList::from("section")]),
    );
    assert_eq!(props.bg_color, None);
}

#[test]
fn style_macro_supports_nested_parent_compounds_and_pseudo_classes() {
    let sheet = style! {
        .item {
            &.selected {
                bg_color: blue;
            }

            &:first-child {
                text_color: red;
            }
        }
    };

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("item selected")).with_child_position(1, 3),
    );
    assert_eq!(props.bg_color, Some(Color::BLUE));
    assert_eq!(props.text_color, Some(Color::RED));

    let props = sheet
        .matched_props(&MatchContext::new(ClassList::from("selected")).with_child_position(1, 3));
    assert_eq!(props.bg_color, None);
    assert_eq!(props.text_color, None);
}

#[test]
fn style_macro_substitutes_parent_references_elsewhere_in_nested_selectors() {
    let sheet = style! {
        .button {
            .theme >> & {
                bg_color: blue;
            }
        }
    };

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("button"))
            .with_ancestors([ClassList::from("section"), ClassList::from("theme")]),
    );
    assert_eq!(props.bg_color, Some(Color::BLUE));

    let props = sheet.matched_props(
        &MatchContext::new(ClassList::from("button")).with_ancestors([ClassList::from("section")]),
    );
    assert_eq!(props.bg_color, None);
}

#[test]
fn style_macro_supports_nested_relative_combinators() {
    let sheet = style! {
        .panel {
            > .button {
                bg_color: red;
            }

            >> .label {
                text_color: blue;
            }
        }

        .label {
            + .input {
                --adjacent: yes;
            }

            ~ .help {
                --subsequent: yes;
            }
        }
    };

    let button = sheet.matched_props(
        &MatchContext::new(ClassList::from("button")).with_ancestors([ClassList::from("panel")]),
    );
    assert_eq!(button.bg_color, Some(Color::RED));

    let label = sheet.matched_props(
        &MatchContext::new(ClassList::from("label"))
            .with_ancestors([ClassList::from("section"), ClassList::from("panel")]),
    );
    assert_eq!(label.text_color, Some(Color::BLUE));

    let input = sheet.matched_props(
        &MatchContext::new(ClassList::from("input"))
            .with_previous_siblings([ClassList::from("label")]),
    );
    assert_eq!(input.custom("--adjacent"), Some("yes"));

    let help = sheet.matched_props(
        &MatchContext::new(ClassList::from("help"))
            .with_previous_siblings([ClassList::from("spacer"), ClassList::from("label")]),
    );
    assert_eq!(help.custom("--subsequent"), Some("yes"));
}

#[test]
fn nested_selector_lists_expand_and_preserve_cascade_behavior() {
    let sheet = style! {
        .panel, .dialog {
            bg_color: red;

            .title, .subtitle {
                text_color: blue;
            }

            &.selected {
                bg_color: blue;
            }
        }

        .dialog.selected {
            bg_color: red;
        }
    };

    let title = sheet.matched_props(
        &MatchContext::new(ClassList::from("subtitle")).with_ancestors([ClassList::from("dialog")]),
    );
    assert_eq!(title.text_color, Some(Color::BLUE));

    let selected = sheet.matched_props(&MatchContext::new(ClassList::from("dialog selected")));
    assert_eq!(selected.bg_color, Some(Color::RED));
}

#[test]
fn declarations_after_nested_rules_stay_on_the_parent_rule() {
    let sheet = style! {
        .panel {
            bg_color: red;

            .child {
                bg_color: blue;
            }

            text_color: blue;
        }
    };

    let panel = sheet.matched_props(&MatchContext::new(ClassList::from("panel")));
    assert_eq!(panel.bg_color, Some(Color::RED));
    assert_eq!(panel.text_color, Some(Color::BLUE));
}
