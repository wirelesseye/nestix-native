use std::rc::Rc;

use nestix::{Element, Layout, component, components::ContextProvider, layout, props};

use crate::{Backend, BackendContext};

#[derive(Clone)]
pub(crate) struct BackendChainContext {
    backends: Rc<[&'static dyn Backend]>,
}

/// Properties for [`BackendProvider`].
#[props]
pub struct BackendProviderProps {
    /// Backend tried before providers inherited from an outer scope.
    #[props(start)]
    pub backend: &'static dyn Backend,

    /// Components rendered with this backend preference.
    #[props(default)]
    pub children: Layout,
}

/// Adds a backend to the front of the descendant backend fallback chain.
///
/// The nearest provider is tried first. When it does not implement a facade
/// component, Nestix Native continues with each inherited provider in order.
#[component]
pub fn BackendProvider(props: &BackendProviderProps, element: &Element) -> Element {
    let backend = props.backend.get();
    let mut backends = vec![backend];

    if let Some(parent) = element.context::<BackendChainContext>() {
        backends.extend(parent.backends.iter().copied());
    } else if let Some(parent) = element.context::<BackendContext>() {
        backends.push(parent.backend);
    }

    layout! {
        ContextProvider<BackendChainContext>(BackendChainContext { backends: backends.into(),  }) {
            ContextProvider<BackendContext>(BackendContext { backend }) {
                $(props.children.clone())
            }
        }
    }
}

/// Returns the nearest backend visible to an element.
#[doc(hidden)]
pub fn active_backend(element: &Element) -> Option<&'static dyn Backend> {
    element
        .context::<BackendChainContext>()
        .and_then(|context| context.backends.first().copied())
        .or_else(|| {
            element
                .context::<BackendContext>()
                .map(|context| context.backend)
        })
}

/// Tries a component factory against the inherited backend chain.
#[doc(hidden)]
pub fn create_backend_element(
    element: &Element,
    component: &'static str,
    mut create: impl FnMut(&'static dyn Backend) -> Option<Element>,
) -> Option<Element> {
    let chain = element.context::<BackendChainContext>();
    let direct = element.context::<BackendContext>();
    let backends: Vec<_> = if let Some(chain) = &chain {
        chain.backends.iter().copied().collect()
    } else if let Some(direct) = &direct {
        vec![direct.backend]
    } else {
        panic!("native components must be mounted beneath Root or BackendProvider");
    };

    for backend in &backends {
        if let Some(output) = create(*backend) {
            return Some(output);
        }
    }

    let backend_ids = backends
        .iter()
        .map(|backend| backend.backend_id())
        .collect::<Vec<_>>()
        .join(", ");
    log::warn!("{component} is not implemented by any backend in [{backend_ids}]");
    None
}
