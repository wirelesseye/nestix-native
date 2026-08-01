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

/// An macOS visual-effect material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MacOSMaterial {
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

/// A Windows system-backdrop material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WindowsMaterial {
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
/// [`Self::platforms`], [`Self::for_macos`], or [`Self::for_windows`] when an
/// exact native material is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Material {
    macos: Option<MacOSMaterial>,
    windows: Option<WindowsMaterial>,
}

impl Material {
    /// Recommended material for a primary application window.
    pub const WINDOW: Self = Self::platforms(MacOSMaterial::WindowBackground, WindowsMaterial::Mica);

    /// Recommended material for a sidebar or navigation pane.
    pub const SIDEBAR: Self = Self::platforms(MacOSMaterial::Sidebar, WindowsMaterial::MicaAlt);

    /// Recommended material for general content placed above a window surface.
    pub const CONTENT: Self =
        Self::platforms(MacOSMaterial::ContentBackground, WindowsMaterial::MicaAlt);

    /// Recommended material for transient UI such as a popover or flyout.
    pub const TRANSIENT: Self = Self::platforms(MacOSMaterial::Popover, WindowsMaterial::Acrylic);

    /// Creates a material with an exact choice for both supported backends.
    pub const fn platforms(macos: MacOSMaterial, windows: WindowsMaterial) -> Self {
        Self {
            macos: Some(macos),
            windows: Some(windows),
        }
    }

    /// Creates a material that is rendered only on macOS.
    pub const fn for_macos(material: MacOSMaterial) -> Self {
        Self {
            macos: Some(material),
            windows: None,
        }
    }

    /// Creates a material that is rendered only on Windows.
    pub const fn for_windows(material: WindowsMaterial) -> Self {
        Self {
            macos: None,
            windows: Some(material),
        }
    }

    /// Returns the native material requested for macOS.
    pub const fn macos_material(self) -> Option<MacOSMaterial> {
        self.macos
    }

    /// Returns the native material requested for Windows.
    pub const fn windows_material(self) -> Option<WindowsMaterial> {
        self.windows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_specific_materials_do_not_imply_a_fallback() {
        let material = Material::for_macos(MacOSMaterial::Sidebar);

        assert_eq!(material.macos_material(), Some(MacOSMaterial::Sidebar));
        assert_eq!(material.windows_material(), None);
    }

    #[test]
    fn semantic_materials_select_both_backends() {
        assert_eq!(
            Material::WINDOW.macos_material(),
            Some(MacOSMaterial::WindowBackground)
        );
        assert_eq!(Material::WINDOW.windows_material(), Some(WindowsMaterial::Mica));
    }
}
