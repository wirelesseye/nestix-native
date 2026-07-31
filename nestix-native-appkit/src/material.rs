use nestix_native_core::{AppKitMaterial, Material, MaterialSource};
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
    let material = native_material(material.appkit_material()?)?;
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

fn native_material(material: AppKitMaterial) -> Option<NSVisualEffectMaterial> {
    Some(match material {
        AppKitMaterial::Titlebar => NSVisualEffectMaterial::Titlebar,
        AppKitMaterial::Selection => NSVisualEffectMaterial::Selection,
        AppKitMaterial::Menu => NSVisualEffectMaterial::Menu,
        AppKitMaterial::Popover => NSVisualEffectMaterial::Popover,
        AppKitMaterial::Sidebar => NSVisualEffectMaterial::Sidebar,
        AppKitMaterial::Header => NSVisualEffectMaterial::HeaderView,
        AppKitMaterial::Sheet => NSVisualEffectMaterial::Sheet,
        AppKitMaterial::WindowBackground => NSVisualEffectMaterial::WindowBackground,
        AppKitMaterial::HudWindow => NSVisualEffectMaterial::HUDWindow,
        AppKitMaterial::FullScreenUi => NSVisualEffectMaterial::FullScreenUI,
        AppKitMaterial::Tooltip => NSVisualEffectMaterial::ToolTip,
        AppKitMaterial::ContentBackground => NSVisualEffectMaterial::ContentBackground,
        AppKitMaterial::UnderWindowBackground => NSVisualEffectMaterial::UnderWindowBackground,
        AppKitMaterial::UnderPageBackground => NSVisualEffectMaterial::UnderPageBackground,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_current_appkit_material() {
        let materials = [
            AppKitMaterial::Titlebar,
            AppKitMaterial::Selection,
            AppKitMaterial::Menu,
            AppKitMaterial::Popover,
            AppKitMaterial::Sidebar,
            AppKitMaterial::Header,
            AppKitMaterial::Sheet,
            AppKitMaterial::WindowBackground,
            AppKitMaterial::HudWindow,
            AppKitMaterial::FullScreenUi,
            AppKitMaterial::Tooltip,
            AppKitMaterial::ContentBackground,
            AppKitMaterial::UnderWindowBackground,
            AppKitMaterial::UnderPageBackground,
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
