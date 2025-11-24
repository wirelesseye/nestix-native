use nestix::{Element, callback, component, components::ContextProvider, layout};
use nestix_native_core::StackViewProps;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::NSView;
use objc2_foundation::NSObjectProtocol;

use crate::ParentViewContext;

#[component]
pub fn AppkitStackView(props: &StackViewProps, element: &Element) -> Element {
    let mtm = MainThreadMarker::new().unwrap();
    let view = NNStackView::new(mtm);

    element.provide_handle(view.as_ref() as *const NSView);

    let parent = element.context::<ParentViewContext>();
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
