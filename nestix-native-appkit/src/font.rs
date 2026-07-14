use nestix_native_core::{FontStyle, FontWeight, ResolvedFontProps};
use objc2::{MainThreadMarker, Message, rc::Retained};
use objc2_app_kit::{NSColor, NSFont, NSFontManager, NSFontTraitMask};
use objc2_foundation::NSString;

pub(crate) fn resolve_font(
    original: &NSFont,
    props: &ResolvedFontProps,
    mtm: MainThreadMarker,
) -> Retained<NSFont> {
    if props.font_family.is_none()
        && props.font_size.is_none()
        && props.font_weight.is_none()
        && props.font_style.is_none()
    {
        return original.retain();
    }
    let manager = NSFontManager::sharedFontManager(mtm);
    let size = props.font_size.unwrap_or_else(|| original.pointSize());
    let mut traits = manager.traitsOfFont(original);
    if let Some(style) = props.font_style {
        match style {
            FontStyle::Normal => traits.remove(NSFontTraitMask::ItalicFontMask),
            FontStyle::Italic => traits.insert(NSFontTraitMask::ItalicFontMask),
        }
    }
    if let Some(weight) = props.font_weight {
        if weight.value() >= FontWeight::Bold.value() {
            traits.insert(NSFontTraitMask::BoldFontMask);
        } else {
            traits.remove(NSFontTraitMask::BoldFontMask);
        }
    }
    let weight = props
        .font_weight
        .map(font_weight)
        .unwrap_or_else(|| manager.weightOfFont(original));
    let requested_family = props
        .font_family
        .as_ref()
        .map(|family| NSString::from_str(family));
    let original_family = original.familyName();
    requested_family
        .as_deref()
        .and_then(|family| manager.fontWithFamily_traits_weight_size(family, traits, weight, size))
        .or_else(|| {
            original_family.as_deref().and_then(|family| {
                manager.fontWithFamily_traits_weight_size(family, traits, weight, size)
            })
        })
        .unwrap_or_else(|| original.fontWithSize(size))
}

pub(crate) fn ns_color(color: nestix_native_core::Color) -> Retained<NSColor> {
    let color = color.into_rgb();
    NSColor::colorWithDeviceRed_green_blue_alpha(
        color.red as f64 / 255.0,
        color.green as f64 / 255.0,
        color.blue as f64 / 255.0,
        color.alpha as f64 / 255.0,
    )
}

fn font_weight(weight: FontWeight) -> isize {
    ((weight.value() as f64 / 1000.0) * 15.0).round() as isize
}
