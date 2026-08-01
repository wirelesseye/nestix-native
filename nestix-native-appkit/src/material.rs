use nestix_native_core::{MacOSMaterial, Material, MaterialSource};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
};

pub(crate) fn visual_effect_view(
    mtm: MainThreadMarker,
    material: Material,
    source: MaterialSource,
    automatic_source: NSVisualEffectBlendingMode,
) -> Option<Retained<NSVisualEffectView>> {
    let material = native_material(material.macos_material()?)?;
    let view = NSVisualEffectView::new(mtm);
    view.setMaterial(material);
    view.setBlendingMode(native_source(source, automatic_source));
    view.setState(NSVisualEffectState::FollowsWindowActiveState);
    Some(view)
}

fn native_source(
    source: MaterialSource,
    automatic_source: NSVisualEffectBlendingMode,
) -> NSVisualEffectBlendingMode {
    match source {
        MaterialSource::Automatic => automatic_source,
        MaterialSource::BehindWindow => NSVisualEffectBlendingMode::BehindWindow,
        MaterialSource::WithinWindow => NSVisualEffectBlendingMode::WithinWindow,
    }
}

fn native_material(material: MacOSMaterial) -> Option<NSVisualEffectMaterial> {
    Some(match material {
        MacOSMaterial::Titlebar => NSVisualEffectMaterial::Titlebar,
        MacOSMaterial::Selection => NSVisualEffectMaterial::Selection,
        MacOSMaterial::Menu => NSVisualEffectMaterial::Menu,
        MacOSMaterial::Popover => NSVisualEffectMaterial::Popover,
        MacOSMaterial::Sidebar => NSVisualEffectMaterial::Sidebar,
        MacOSMaterial::Header => NSVisualEffectMaterial::HeaderView,
        MacOSMaterial::Sheet => NSVisualEffectMaterial::Sheet,
        MacOSMaterial::WindowBackground => NSVisualEffectMaterial::WindowBackground,
        MacOSMaterial::HudWindow => NSVisualEffectMaterial::HUDWindow,
        MacOSMaterial::FullScreenUi => NSVisualEffectMaterial::FullScreenUI,
        MacOSMaterial::Tooltip => NSVisualEffectMaterial::ToolTip,
        MacOSMaterial::ContentBackground => NSVisualEffectMaterial::ContentBackground,
        MacOSMaterial::UnderWindowBackground => NSVisualEffectMaterial::UnderWindowBackground,
        MacOSMaterial::UnderPageBackground => NSVisualEffectMaterial::UnderPageBackground,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_current_appkit_material() {
        let materials = [
            MacOSMaterial::Titlebar,
            MacOSMaterial::Selection,
            MacOSMaterial::Menu,
            MacOSMaterial::Popover,
            MacOSMaterial::Sidebar,
            MacOSMaterial::Header,
            MacOSMaterial::Sheet,
            MacOSMaterial::WindowBackground,
            MacOSMaterial::HudWindow,
            MacOSMaterial::FullScreenUi,
            MacOSMaterial::Tooltip,
            MacOSMaterial::ContentBackground,
            MacOSMaterial::UnderWindowBackground,
            MacOSMaterial::UnderPageBackground,
        ];

        assert!(
            materials
                .into_iter()
                .all(|material| native_material(material).is_some())
        );
    }

    #[test]
    fn automatic_source_uses_the_scope_default() {
        assert_eq!(
            native_source(
                MaterialSource::Automatic,
                NSVisualEffectBlendingMode::WithinWindow,
            ),
            NSVisualEffectBlendingMode::WithinWindow
        );
        assert_eq!(
            native_source(
                MaterialSource::Automatic,
                NSVisualEffectBlendingMode::BehindWindow,
            ),
            NSVisualEffectBlendingMode::BehindWindow
        );
    }

    #[test]
    fn explicit_source_overrides_the_scope_default() {
        assert_eq!(
            native_source(
                MaterialSource::BehindWindow,
                NSVisualEffectBlendingMode::WithinWindow,
            ),
            NSVisualEffectBlendingMode::BehindWindow
        );
        assert_eq!(
            native_source(
                MaterialSource::WithinWindow,
                NSVisualEffectBlendingMode::BehindWindow,
            ),
            NSVisualEffectBlendingMode::WithinWindow
        );
    }
}
