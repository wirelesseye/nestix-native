use nestix::{Element, create_element};
use nestix_native_core::{Backend, ButtonProps, FlexViewProps, InputProps, TextProps};

use crate::{Button, FlexView, Input, Text};

/// Stable identifier for the backend used by managed `DomSurface` documents.
pub const DOM_SURFACE_BACKEND_ID: &str = "nestix-native-dom-surface";

/// Shared backend instance used by managed DOM surfaces.
pub const DOM_SURFACE_BACKEND: DomSurfaceBackend = DomSurfaceBackend;

/// Backend for components rendered into a managed `DomSurface` document.
///
/// Its capability list is intentionally independent from [`crate::DomBackend`]
/// so browser and embedded DOM rendering can diverge without changing the
/// surface host.
pub struct DomSurfaceBackend;

impl Backend for DomSurfaceBackend {
    fn backend_id(&self) -> &'static str {
        DOM_SURFACE_BACKEND_ID
    }

    fn create_flex_view(&self, props: FlexViewProps) -> Option<Element> {
        Some(create_element::<FlexView>(props))
    }

    fn create_text(&self, props: TextProps) -> Option<Element> {
        Some(create_element::<Text>(props))
    }

    fn create_button(&self, props: ButtonProps) -> Option<Element> {
        Some(create_element::<Button>(props))
    }

    fn create_input(&self, props: InputProps) -> Option<Element> {
        Some(create_element::<Input>(props))
    }
}
