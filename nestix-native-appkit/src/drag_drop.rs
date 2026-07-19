use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::CString,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use nestix::{Element, PropValue, Shared, callback, closure, component, layout, scoped_effect};
use nestix_native_core::{
    DragContent, DragDataType, DragDataTypes, DragFilesCallback, DragImage, DragImageCallback,
    DragModifiers, DragOffer, DragOperation, DragOperations, DragReadError, DragSourceError,
    DragSourceOutcome, DragSourceProps, DropDataProvider, DropDataReader, DropEvent,
    DropTargetProps,
};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Bool, ClassBuilder, NSObject, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{
    NSDragOperation, NSDraggingInfo, NSDraggingItem, NSDraggingSession, NSDraggingSource, NSEvent,
    NSEventModifierFlags, NSFilePromiseProvider, NSFilePromiseProviderDelegate,
    NSGestureRecognizerState, NSPanGestureRecognizer, NSPasteboard, NSPasteboardItem,
    NSPasteboardTypeFileURL, NSPasteboardTypePNG, NSPasteboardTypeString, NSPasteboardTypeTIFF,
    NSView,
};
use objc2_foundation::{NSArray, NSCopying, NSData, NSObjectProtocol, NSPoint, NSString, NSURL};

thread_local! {
    static DROP_TARGETS: RefCell<HashMap<usize, Vec<Rc<DropTargetState>>>> =
        RefCell::new(HashMap::new());
}

static NEXT_CLASS_ID: AtomicU64 = AtomicU64::new(1);

struct DropTargetState {
    id: u64,
    view: Retained<NSView>,
    enabled: PropValue<bool>,
    accepted_types: PropValue<DragDataTypes>,
    default_operation: PropValue<DragOperation>,
    on_enter: PropValue<Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>>,
    on_over: PropValue<Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>>,
    on_leave: PropValue<Option<Shared<dyn Fn()>>>,
    on_drop: PropValue<Shared<dyn Fn(DropEvent)>>,
    last_operation: RefCell<Option<DragOperation>>,
}

fn view_key(object: &AnyObject) -> usize {
    object as *const AnyObject as usize
}

fn active_target(object: &AnyObject) -> Option<Rc<DropTargetState>> {
    DROP_TARGETS.with_borrow(|targets| {
        targets
            .get(&view_key(object))
            .and_then(|targets| targets.last())
            .cloned()
    })
}

fn native_operations(value: DragOperations) -> NSDragOperation {
    let mut result = NSDragOperation::None;
    if value.contains(DragOperations::COPY) {
        result |= NSDragOperation::Copy;
    }
    if value.contains(DragOperations::MOVE) {
        result |= NSDragOperation::Move;
    }
    if value.contains(DragOperations::LINK) {
        result |= NSDragOperation::Link;
    }
    result
}

fn operations_from_native(value: NSDragOperation) -> DragOperations {
    let mut result = DragOperations::NONE;
    if value.contains(NSDragOperation::Copy) {
        result |= DragOperations::COPY;
    }
    if value.contains(NSDragOperation::Move) {
        result |= DragOperations::MOVE;
    }
    if value.contains(NSDragOperation::Link) {
        result |= DragOperations::LINK;
    }
    result
}

fn native_operation(value: Option<DragOperation>) -> NSDragOperation {
    match value {
        Some(DragOperation::Copy) => NSDragOperation::Copy,
        Some(DragOperation::Move) => NSDragOperation::Move,
        Some(DragOperation::Link) => NSDragOperation::Link,
        None => NSDragOperation::None,
    }
}

fn operation_from_native(value: NSDragOperation) -> Option<DragOperation> {
    if value.contains(NSDragOperation::Move) {
        Some(DragOperation::Move)
    } else if value.contains(NSDragOperation::Copy) {
        Some(DragOperation::Copy)
    } else if value.contains(NSDragOperation::Link) {
        Some(DragOperation::Link)
    } else {
        None
    }
}

fn modifiers() -> DragModifiers {
    let flags = NSEvent::modifierFlags_class();
    let mut result = DragModifiers::NONE;
    if flags.contains(NSEventModifierFlags::Command) {
        result |= DragModifiers::PRIMARY;
    }
    if flags.contains(NSEventModifierFlags::Shift) {
        result |= DragModifiers::SHIFT;
    }
    if flags.contains(NSEventModifierFlags::Option) {
        result |= DragModifiers::ALT;
    }
    result
}

fn pasteboard_types(pasteboard: &NSPasteboard) -> DragDataTypes {
    let Some(types) = pasteboard.types() else {
        return DragDataTypes::NONE;
    };
    let mut result = DragDataTypes::NONE;
    for kind in types.iter() {
        let kind: &NSString = &kind;
        if kind == unsafe { NSPasteboardTypeFileURL } {
            result |= DragDataTypes::FILES;
        } else if kind == unsafe { NSPasteboardTypePNG }
            || kind == unsafe { NSPasteboardTypeTIFF }
            || matches!(
                kind.to_string().as_str(),
                "public.jpeg" | "com.compuserve.gif"
            )
            || kind.to_string().starts_with("image/")
        {
            result |= DragDataTypes::IMAGE;
        } else if kind == unsafe { NSPasteboardTypeString } {
            result |= DragDataTypes::TEXT;
        }
    }
    result
}

fn drag_offer(state: &DropTargetState, sender: &ProtocolObject<dyn NSDraggingInfo>) -> DragOffer {
    let point = state
        .view
        .convertPoint_fromView(sender.draggingLocation(), None);
    DragOffer {
        available_types: pasteboard_types(&sender.draggingPasteboard())
            .intersection(state.accepted_types.get()),
        allowed_operations: operations_from_native(sender.draggingSourceOperationMask()),
        position: nestix_native_core::dpi::LogicalPosition::new(
            point.x,
            state.view.bounds().size.height - point.y,
        ),
        modifiers: modifiers(),
    }
}

fn decide_operation(
    state: &DropTargetState,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
    entering: bool,
) -> Option<DragOperation> {
    if !state.enabled.get() {
        return None;
    }
    let offer = drag_offer(state, sender);
    if offer.available_types.is_empty() {
        return None;
    }
    let callback = if entering {
        state.on_enter.get()
    } else {
        state.on_over.get()
    };
    let operation = callback
        .as_ref()
        .and_then(|callback| callback(&offer))
        .or_else(|| callback.is_none().then(|| state.default_operation.get()));
    operation.filter(|operation| offer.allowed_operations.contains_operation(*operation))
}

extern "C-unwind" fn dragging_entered(
    object: &AnyObject,
    _: Sel,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> NSDragOperation {
    let Some(state) = active_target(object) else {
        return NSDragOperation::None;
    };
    let operation = decide_operation(&state, sender, true);
    state.last_operation.replace(operation);
    native_operation(operation)
}

extern "C-unwind" fn dragging_updated(
    object: &AnyObject,
    _: Sel,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> NSDragOperation {
    let Some(state) = active_target(object) else {
        return NSDragOperation::None;
    };
    let operation = decide_operation(&state, sender, false);
    state.last_operation.replace(operation);
    native_operation(operation)
}

extern "C-unwind" fn dragging_exited(
    object: &AnyObject,
    _: Sel,
    _: Option<&ProtocolObject<dyn NSDraggingInfo>>,
) {
    if let Some(state) = active_target(object) {
        state.last_operation.replace(None);
        if let Some(callback) = state.on_leave.get() {
            callback();
        }
    }
}

extern "C-unwind" fn prepare_for_drag_operation(
    object: &AnyObject,
    _: Sel,
    _: &ProtocolObject<dyn NSDraggingInfo>,
) -> Bool {
    Bool::new(active_target(object).is_some_and(|state| state.last_operation.borrow().is_some()))
}

extern "C-unwind" fn perform_drag_operation(
    object: &AnyObject,
    _: Sel,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> Bool {
    let Some(state) = active_target(object) else {
        return Bool::NO;
    };
    let Some(operation) = *state.last_operation.borrow() else {
        return Bool::NO;
    };
    let offer = drag_offer(&state, sender);
    let pasteboard = sender.draggingPasteboard();
    let reader = reader_for_pasteboard(pasteboard, offer.available_types);
    (state.on_drop.get())(DropEvent {
        operation,
        position: offer.position,
        modifiers: offer.modifiers,
        data: reader,
    });
    state.last_operation.replace(None);
    Bool::YES
}

fn reader_for_pasteboard(
    pasteboard: Retained<NSPasteboard>,
    available_types: DragDataTypes,
) -> DropDataReader {
    DropDataReader::new(DropDataProvider {
        available_types,
        read_files: callback!([pasteboard] |callback: DragFilesCallback| {
            let result = read_files(&pasteboard);
            callback(result);
        }),
        read_image: callback!([pasteboard] |callback: DragImageCallback| {
            let result = read_image(&pasteboard);
            callback(result);
        }),
        read_text: callback!([pasteboard] |callback: nestix_native_core::DragTextCallback| {
            let result = pasteboard
                .stringForType(unsafe { NSPasteboardTypeString })
                .map(|value| value.to_string())
                .ok_or(DragReadError::Unavailable(DragDataType::Text));
            callback(result);
        }),
    })
}

fn read_files(pasteboard: &NSPasteboard) -> Result<Vec<PathBuf>, DragReadError> {
    let mut paths = Vec::new();
    if let Some(items) = pasteboard.pasteboardItems() {
        for item in items.iter() {
            let Some(value) = item.stringForType(unsafe { NSPasteboardTypeFileURL }) else {
                continue;
            };
            let Some(url) = NSURL::URLWithString(&value) else {
                continue;
            };
            if let Some(path) = url.path() {
                paths.push(PathBuf::from(path.to_string()));
            }
        }
    }
    if paths.is_empty() {
        Err(DragReadError::Unavailable(DragDataType::Files))
    } else {
        Ok(paths)
    }
}

fn read_image(pasteboard: &NSPasteboard) -> Result<DragImage, DragReadError> {
    let jpeg = NSString::from_str("public.jpeg");
    let gif = NSString::from_str("com.compuserve.gif");
    let candidates = [
        (unsafe { NSPasteboardTypePNG }, "image/png", "image.png"),
        (unsafe { NSPasteboardTypeTIFF }, "image/tiff", "image.tiff"),
        (jpeg.as_ref(), "image/jpeg", "image.jpg"),
        (gif.as_ref(), "image/gif", "image.gif"),
    ];
    for (kind, media_type, suggested_name) in candidates {
        if let Some(data) = pasteboard.dataForType(kind) {
            return Ok(DragImage::new(data.to_vec(), media_type, suggested_name));
        }
    }
    Err(DragReadError::Unavailable(DragDataType::Image))
}

struct DropRegistration {
    id: u64,
    view: Retained<NSView>,
    previous_class: &'static AnyClass,
    installed_class: &'static AnyClass,
}

impl Drop for DropRegistration {
    fn drop(&mut self) {
        let key = self.view.as_ref() as *const NSView as usize;
        let is_last = DROP_TARGETS.with_borrow_mut(|targets| {
            if let Some(stack) = targets.get_mut(&key) {
                stack.retain(|state| state.id != self.id);
                if stack.is_empty() {
                    targets.remove(&key);
                    return true;
                }
            }
            false
        });
        let object = self.view.as_ref() as *const NSView as *mut AnyObject;
        unsafe {
            if (&*object).class() == self.installed_class {
                objc2::ffi::object_setClass(object, self.previous_class);
            }
        }
        if is_last {
            self.view.unregisterDraggedTypes();
        }
    }
}

fn register_drop_target(state: Rc<DropTargetState>) -> DropRegistration {
    let view = state.view.clone();
    let previous_class = view.class();
    let class_id = NEXT_CLASS_ID.fetch_add(1, Ordering::Relaxed);
    let name = CString::new(format!("NestixDropTarget_{class_id}")).unwrap();
    let mut builder = ClassBuilder::new(&name, previous_class).unwrap();
    unsafe {
        builder.add_method(
            sel!(draggingEntered:),
            dragging_entered as extern "C-unwind" fn(_, _, _) -> _,
        );
        builder.add_method(
            sel!(draggingUpdated:),
            dragging_updated as extern "C-unwind" fn(_, _, _) -> _,
        );
        builder.add_method(
            sel!(draggingExited:),
            dragging_exited as extern "C-unwind" fn(_, _, _),
        );
        builder.add_method(
            sel!(prepareForDragOperation:),
            prepare_for_drag_operation as extern "C-unwind" fn(_, _, _) -> _,
        );
        builder.add_method(
            sel!(performDragOperation:),
            perform_drag_operation as extern "C-unwind" fn(_, _, _) -> _,
        );
    }
    let installed_class = builder.register();
    let object = view.as_ref() as *const NSView as *mut AnyObject;
    unsafe {
        objc2::ffi::object_setClass(object, installed_class);
    }
    DROP_TARGETS.with_borrow_mut(|targets| {
        targets
            .entry(view_key(unsafe { &*object }))
            .or_default()
            .push(state.clone());
    });
    let types = NSArray::from_retained_slice(&[
        unsafe { NSPasteboardTypeFileURL }.copy(),
        unsafe { NSPasteboardTypePNG }.copy(),
        unsafe { NSPasteboardTypeTIFF }.copy(),
        unsafe { NSPasteboardTypeString }.copy(),
        NSString::from_str("public.jpeg"),
        NSString::from_str("com.compuserve.gif"),
    ]);
    view.registerForDraggedTypes(&types);
    DropRegistration {
        id: state.id,
        view,
        previous_class,
        installed_class,
    }
}

#[component]
pub fn DropTarget(props: &DropTargetProps, element: &Element) -> Element {
    let registration = Rc::new(RefCell::new(None::<DropRegistration>));
    let enabled = props.enabled.clone();
    let accepted_types = props.accepted_types.clone();
    let default_operation = props.default_operation.clone();
    let on_enter = props.on_enter.clone();
    let on_over = props.on_over.clone();
    let on_leave = props.on_leave.clone();
    let on_drop = props.on_drop.clone();
    scoped_effect!(
        element,
        [registration, props.children] || {
            registration.borrow_mut().take();
            children.get().on_last_handle_change(closure!(
                [
                    registration,
                    enabled,
                    accepted_types,
                    default_operation,
                    on_enter,
                    on_over,
                    on_leave,
                    on_drop
                ] | handle
                    | {
                        registration.borrow_mut().take();
                        let Some(handle) = handle else { return };
                        let Some(pointer) = handle.downcast_ref::<*const NSObject>() else {
                            return;
                        };
                        let Some(view) = (unsafe { &**pointer }).downcast_ref::<NSView>() else {
                            return;
                        };
                        let id = NEXT_CLASS_ID.fetch_add(1, Ordering::Relaxed);
                        let state = Rc::new(DropTargetState {
                            id,
                            view: view.retain(),
                            enabled: enabled.clone(),
                            accepted_types: accepted_types.clone(),
                            default_operation: default_operation.clone(),
                            on_enter: on_enter.clone(),
                            on_over: on_over.clone(),
                            on_leave: on_leave.clone(),
                            on_drop: on_drop.clone(),
                            last_operation: RefCell::new(None),
                        });
                        registration
                            .borrow_mut()
                            .replace(register_drop_target(state));
                    }
            ));
        }
    );
    element.on_unmount(closure!(
        [registration] || {
            registration.borrow_mut().take();
        }
    ));
    layout! {
        $(props.children.get())
    }
}

struct DragSourceState {
    view: Retained<NSView>,
    content: PropValue<DragContent>,
    enabled: PropValue<bool>,
    allowed_operations: PropValue<DragOperations>,
    on_started: PropValue<Option<Shared<dyn Fn()>>>,
    on_completed: PropValue<Option<Shared<dyn Fn(DragSourceOutcome)>>>,
    on_error: PropValue<Option<Shared<dyn Fn(DragSourceError)>>>,
    promise_delegates: RefCell<Vec<Retained<FilePromiseDelegate>>>,
}

struct FilePromiseState {
    name: String,
    bytes: Arc<[u8]>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixFilePromiseDelegate"]
    #[ivars = FilePromiseState]
    struct FilePromiseDelegate;

    unsafe impl NSObjectProtocol for FilePromiseDelegate {}

    unsafe impl NSFilePromiseProviderDelegate for FilePromiseDelegate {
        #[unsafe(method_id(filePromiseProvider:fileNameForType:))]
        fn file_name(&self, _: &NSFilePromiseProvider, _: &NSString) -> Retained<NSString> {
            NSString::from_str(&self.ivars().name)
        }

        #[unsafe(method(filePromiseProvider:writePromiseToURL:completionHandler:))]
        fn write_promise(
            &self,
            _: &NSFilePromiseProvider,
            url: &NSURL,
            completion: &block2::DynBlock<dyn Fn(*mut objc2_foundation::NSError)>,
        ) {
            let _ = NSData::with_bytes(&self.ivars().bytes).writeToURL_atomically(url, true);
            completion.call((std::ptr::null_mut(),));
        }
    }
);

impl FilePromiseDelegate {
    fn new(mtm: MainThreadMarker, state: FilePromiseState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NestixDragSourceHandler"]
    #[ivars = DragSourceState]
    struct DragSourceHandler;

    unsafe impl NSObjectProtocol for DragSourceHandler {}

    unsafe impl NSDraggingSource for DragSourceHandler {
        #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
        fn source_operation_mask(
            &self,
            _: &NSDraggingSession,
            _: objc2_app_kit::NSDraggingContext,
        ) -> NSDragOperation {
            native_operations(self.ivars().allowed_operations.get())
        }

        #[unsafe(method(draggingSession:endedAtPoint:operation:))]
        fn ended(&self, _: &NSDraggingSession, _: NSPoint, operation: NSDragOperation) {
            self.ivars().promise_delegates.borrow_mut().clear();
            if let Some(callback) = self.ivars().on_completed.get() {
                callback(match operation_from_native(operation) {
                    Some(operation) => DragSourceOutcome::Dropped(operation),
                    None => DragSourceOutcome::Cancelled,
                });
            }
        }
    }

    impl DragSourceHandler {
        #[unsafe(method(handlePan:))]
        fn handle_pan(&self, recognizer: &NSPanGestureRecognizer) {
            if recognizer.state() != NSGestureRecognizerState::Began || !self.ivars().enabled.get() {
                return;
            }
            self.begin_drag();
        }
    }
);

impl DragSourceHandler {
    fn new(mtm: MainThreadMarker, state: DragSourceState) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(state);
        unsafe { msg_send![super(this), init] }
    }

    fn begin_drag(&self) {
        let content = self.ivars().content.get();
        if content.is_empty() {
            if let Some(callback) = self.ivars().on_error.get() {
                callback(DragSourceError::EmptyContent);
            }
            return;
        }
        let Some(event) = self
            .ivars()
            .view
            .window()
            .and_then(|window| window.currentEvent())
        else {
            if let Some(callback) = self.ivars().on_error.get() {
                callback(DragSourceError::Backend("no current pointer event".into()));
            }
            return;
        };
        let (items, delegates) = dragging_items(&content, self.ivars().view.bounds());
        if items.is_empty() {
            if let Some(callback) = self.ivars().on_error.get() {
                callback(DragSourceError::EmptyContent);
            }
            return;
        }
        if let Some(callback) = self.ivars().on_started.get() {
            callback();
        }
        self.ivars().promise_delegates.replace(delegates);
        let source = ProtocolObject::from_ref(self);
        self.ivars()
            .view
            .beginDraggingSessionWithItems_event_source(&items, &event, source);
    }
}

fn dragging_items(
    content: &DragContent,
    frame: objc2_foundation::NSRect,
) -> (
    Retained<NSArray<NSDraggingItem>>,
    Vec<Retained<FilePromiseDelegate>>,
) {
    let mut items = Vec::new();
    let mut delegates = Vec::new();
    if let Some(files) = content.files() {
        for path in files {
            let item = NSPasteboardItem::new();
            let path = NSString::from_str(&path.to_string_lossy());
            let url = NSURL::fileURLWithPath(&path);
            if let Some(value) = url.absoluteString() {
                item.setString_forType(&value, unsafe { NSPasteboardTypeFileURL });
                let dragging = NSDraggingItem::initWithPasteboardWriter(
                    NSDraggingItem::alloc(),
                    ProtocolObject::from_ref(&*item),
                );
                unsafe { dragging.setDraggingFrame_contents(frame, None) };
                items.push(dragging);
            }
        }
    }
    if content.image().is_some() || content.text().is_some() {
        let item = NSPasteboardItem::new();
        if let Some(text) = content.text() {
            item.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
        }
        if let Some(image) = content.image() {
            let kind = image_pasteboard_type(&image.media_type);
            item.setData_forType(&NSData::with_bytes(&image.bytes), &kind);
        }
        let dragging = NSDraggingItem::initWithPasteboardWriter(
            NSDraggingItem::alloc(),
            ProtocolObject::from_ref(&*item),
        );
        unsafe { dragging.setDraggingFrame_contents(frame, None) };
        items.push(dragging);
    }
    if let Some(image) = content.image() {
        add_file_promise(
            &mut items,
            &mut delegates,
            frame,
            image_file_type(&image.media_type),
            image.suggested_name.clone(),
            image.bytes.clone(),
        );
    }
    if let Some(text) = content.text() {
        add_file_promise(
            &mut items,
            &mut delegates,
            frame,
            "public.utf8-plain-text",
            "nestix.txt".to_string(),
            Arc::from(text.as_bytes()),
        );
    }
    (NSArray::from_retained_slice(&items), delegates)
}

fn image_pasteboard_type(media_type: &str) -> Retained<NSString> {
    match media_type.to_ascii_lowercase().as_str() {
        "image/png" => unsafe { NSPasteboardTypePNG }.copy(),
        "image/tiff" => unsafe { NSPasteboardTypeTIFF }.copy(),
        "image/jpeg" | "image/jpg" => NSString::from_str("public.jpeg"),
        "image/gif" => NSString::from_str("com.compuserve.gif"),
        _ => NSString::from_str(media_type),
    }
}

fn image_file_type(media_type: &str) -> &str {
    match media_type.to_ascii_lowercase().as_str() {
        "image/png" => "public.png",
        "image/tiff" => "public.tiff",
        "image/jpeg" | "image/jpg" => "public.jpeg",
        "image/gif" => "com.compuserve.gif",
        _ => media_type,
    }
}

fn add_file_promise(
    items: &mut Vec<Retained<NSDraggingItem>>,
    delegates: &mut Vec<Retained<FilePromiseDelegate>>,
    frame: objc2_foundation::NSRect,
    file_type: &str,
    name: String,
    bytes: Arc<[u8]>,
) {
    let delegate = FilePromiseDelegate::new(
        MainThreadMarker::new().unwrap(),
        FilePromiseState { name, bytes },
    );
    let provider = NSFilePromiseProvider::initWithFileType_delegate(
        NSFilePromiseProvider::alloc(),
        &NSString::from_str(file_type),
        ProtocolObject::from_ref(&*delegate),
    );
    let dragging = NSDraggingItem::initWithPasteboardWriter(
        NSDraggingItem::alloc(),
        ProtocolObject::from_ref(&*provider),
    );
    unsafe { dragging.setDraggingFrame_contents(frame, None) };
    items.push(dragging);
    delegates.push(delegate);
}

struct SourceRegistration {
    view: Retained<NSView>,
    recognizer: Retained<NSPanGestureRecognizer>,
    _handler: Retained<DragSourceHandler>,
}

impl Drop for SourceRegistration {
    fn drop(&mut self) {
        self.view.removeGestureRecognizer(&self.recognizer);
    }
}

#[component]
pub fn DragSource(props: &DragSourceProps, element: &Element) -> Element {
    let registration = Rc::new(RefCell::new(None::<SourceRegistration>));
    scoped_effect!(
        element,
        [
            registration,
            props.children,
            props.content,
            props.enabled,
            props.allowed_operations,
            props.on_started,
            props.on_completed,
            props.on_error
        ] || {
            registration.borrow_mut().take();
            children.get().on_last_handle_change(closure!(
                [
                    registration,
                    content,
                    enabled,
                    allowed_operations,
                    on_started,
                    on_completed,
                    on_error
                ] | handle
                    | {
                        registration.borrow_mut().take();
                        let Some(handle) = handle else { return };
                        let Some(pointer) = handle.downcast_ref::<*const NSObject>() else {
                            return;
                        };
                        let Some(view) = (unsafe { &**pointer }).downcast_ref::<NSView>() else {
                            return;
                        };
                        let mtm = MainThreadMarker::new().unwrap();
                        let handler = DragSourceHandler::new(
                            mtm,
                            DragSourceState {
                                view: view.retain(),
                                content: content.clone(),
                                enabled: enabled.clone(),
                                allowed_operations: allowed_operations.clone(),
                                on_started: on_started.clone(),
                                on_completed: on_completed.clone(),
                                on_error: on_error.clone(),
                                promise_delegates: RefCell::new(Vec::new()),
                            },
                        );
                        let recognizer = unsafe {
                            NSPanGestureRecognizer::initWithTarget_action(
                                NSPanGestureRecognizer::alloc(mtm),
                                Some(handler.as_ref()),
                                Some(sel!(handlePan:)),
                            )
                        };
                        recognizer.setButtonMask(1);
                        view.addGestureRecognizer(&recognizer);
                        registration.borrow_mut().replace(SourceRegistration {
                            view: view.retain(),
                            recognizer,
                            _handler: handler,
                        });
                    }
            ));
        }
    );
    element.on_unmount(closure!(
        [registration] || {
            registration.borrow_mut().take();
        }
    ));
    layout! {
        $(props.children.get())
    }
}
