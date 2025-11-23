use nestix::{
    Element, Shared, callback, component, components::ContextProvider, derive_props, layout,
    provide_handle, use_context,
};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::NSView;
use objc2_foundation::NSObjectProtocol;

#[derive_props]
pub struct AppkitStackViewProps {
    children: Option<Vec<Element>>,
}

#[derive(Clone)]
pub struct ParentViewContext {
    pub add_child: Shared<dyn Fn(&NSView)>,
}

#[component]
pub fn AppkitStackView(props: &AppkitStackViewProps) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let view = NNStackView::new(mtm);

    provide_handle(view.as_ref() as *const NSView);

    let parent = use_context::<ParentViewContext>();
    if let Some(parent) = parent {
        (parent.add_child)(&view);
    }

    layout! {
        ContextProvider<ParentViewContext>(
            .value = ParentViewContext {
                add_child: callback!(view => |subview: &NSView| {
                    view.addSubview(subview);
                })
            },
            .children = props.children.clone(),
        )
    }
}

define_class! {
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[derive(Debug)]
    pub struct NNStackView;

    unsafe impl NSObjectProtocol for NNStackView {}

    impl NNStackView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }
}

impl NNStackView {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
