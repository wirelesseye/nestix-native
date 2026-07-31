/// Selects which content a material samples to produce its visual effect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MaterialSource {
    /// Use the natural source for the material's scope: behind the window for
    /// windows and within the window for bounded areas.
    #[default]
    Automatic,
    /// Sample the desktop and other windows behind the containing window.
    BehindWindow,
    /// Sample content behind the material area within the containing window.
    WithinWindow,
}

/// An AppKit visual-effect material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AppKitMaterial {
    /// Material used by window title bars.
    Titlebar,
    /// Material used to indicate a selection.
    Selection,
    /// Material used by menus.
    Menu,
    /// Material used by popover windows.
    Popover,
    /// Material used by window sidebars.
    Sidebar,
    /// Material used by inline headers and footers.
    Header,
    /// Material used by sheet windows.
    Sheet,
    /// Material used by opaque window backgrounds.
    WindowBackground,
    /// Material used by heads-up display windows.
    HudWindow,
    /// Material used by full-screen modal interfaces.
    FullScreenUi,
    /// Material used by tooltips.
    Tooltip,
    /// Material used by opaque content backgrounds.
    ContentBackground,
    /// Material used beneath a window background.
    UnderWindowBackground,
    /// Material used behind document pages.
    UnderPageBackground,
}

/// A WinUI system-backdrop material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WinUiMaterial {
    /// The standard Mica backdrop.
    Mica,
    /// The alternative Mica backdrop for deeper visual layering.
    MicaAlt,
    /// The standard desktop Acrylic backdrop.
    Acrylic,
    /// The thinner, more transparent desktop Acrylic backdrop.
    AcrylicThin,
}

/// A platform-dependent composited background material.
///
/// The predefined constants provide portable semantic choices. Use
/// [`Self::platforms`], [`Self::for_appkit`], or [`Self::for_winui`] when an
/// exact native material is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Material {
    appkit: Option<AppKitMaterial>,
    winui: Option<WinUiMaterial>,
}

impl Material {
    /// Recommended material for a primary application window.
    pub const WINDOW: Self = Self::platforms(AppKitMaterial::WindowBackground, WinUiMaterial::Mica);

    /// Recommended material for a sidebar or navigation pane.
    pub const SIDEBAR: Self = Self::platforms(AppKitMaterial::Sidebar, WinUiMaterial::MicaAlt);

    /// Recommended material for general content placed above a window surface.
    pub const CONTENT: Self =
        Self::platforms(AppKitMaterial::ContentBackground, WinUiMaterial::MicaAlt);

    /// Recommended material for transient UI such as a popover or flyout.
    pub const TRANSIENT: Self = Self::platforms(AppKitMaterial::Popover, WinUiMaterial::Acrylic);

    /// Creates a material with an exact choice for both supported backends.
    pub const fn platforms(appkit: AppKitMaterial, winui: WinUiMaterial) -> Self {
        Self {
            appkit: Some(appkit),
            winui: Some(winui),
        }
    }

    /// Creates a material that is rendered only by the AppKit backend.
    pub const fn for_appkit(material: AppKitMaterial) -> Self {
        Self {
            appkit: Some(material),
            winui: None,
        }
    }

    /// Creates a material that is rendered only by the WinUI backend.
    pub const fn for_winui(material: WinUiMaterial) -> Self {
        Self {
            appkit: None,
            winui: Some(material),
        }
    }

    /// Returns the native material requested for AppKit.
    pub const fn appkit_material(self) -> Option<AppKitMaterial> {
        self.appkit
    }

    /// Returns the native material requested for WinUI.
    pub const fn winui_material(self) -> Option<WinUiMaterial> {
        self.winui
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_specific_materials_do_not_imply_a_fallback() {
        let material = Material::for_appkit(AppKitMaterial::Sidebar);

        assert_eq!(material.appkit_material(), Some(AppKitMaterial::Sidebar));
        assert_eq!(material.winui_material(), None);
    }

    #[test]
    fn semantic_materials_select_both_backends() {
        assert_eq!(
            Material::WINDOW.appkit_material(),
            Some(AppKitMaterial::WindowBackground)
        );
        assert_eq!(Material::WINDOW.winui_material(), Some(WinUiMaterial::Mica));
    }
}
