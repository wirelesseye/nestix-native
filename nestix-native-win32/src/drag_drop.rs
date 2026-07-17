use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ffi::c_void,
    mem::{ManuallyDrop, size_of},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

use nestix::{Element, PropValue, Shared, callback, closure, component, layout, scoped_effect};
use nestix_native_core::{
    DragContent, DragDataType, DragDataTypes, DragFilesCallback, DragImage, DragImageCallback,
    DragModifiers, DragOffer, DragOperation, DragOperations, DragReadError, DragSourceError,
    DragSourceOutcome, DragSourceProps, DragTextCallback, DropDataProvider, DropDataReader,
    DropEvent, DropTargetProps,
};
use windows::{
    Win32::{
        Foundation::{
            DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DV_E_FORMATETC, E_NOTIMPL, E_POINTER, HWND, LPARAM,
            LRESULT, POINT, POINTL, WPARAM,
        },
        Graphics::Gdi::ScreenToClient,
        System::{
            Com::{
                DVASPECT_CONTENT, FORMATETC, IAdviseSink, IDataObject, IDataObject_Impl,
                IEnumFORMATETC, IEnumSTATDATA, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
            },
            DataExchange::RegisterClipboardFormatW,
            Memory::{
                GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
            },
            Ole::{
                CF_DIB, CF_DIBV5, CF_HDROP, CF_UNICODETEXT, DROPEFFECT, DROPEFFECT_COPY,
                DROPEFFECT_LINK, DROPEFFECT_MOVE, DROPEFFECT_NONE, DoDragDrop, IDropSource,
                IDropSource_Impl, IDropTarget, IDropTarget_Impl, RegisterDragDrop,
                ReleaseStgMedium, RevokeDragDrop,
            },
            SystemServices::{MK_CONTROL, MK_LBUTTON, MK_SHIFT, MODIFIERKEYS_FLAGS},
        },
        UI::{
            HiDpi::GetDpiForWindow,
            Shell::{
                DefSubclassProc, DragQueryFileW, HDROP, RemoveWindowSubclass,
                SHCreateStdEnumFmtEtc, SetWindowSubclass,
            },
            WindowsAndMessaging::{
                GetSystemMetrics, SM_CXDRAG, SM_CYDRAG, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
            },
        },
    },
    core::{Error, HRESULT, HSTRING, Ref, implement},
};

const SOURCE_SUBCLASS_ID: usize = 0x4e65737444726167;
const FD_ATTRIBUTES: u32 = 0x0000_0004;
const FD_FILESIZE: u32 = 0x0000_0040;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

thread_local! {
    static TARGETS: RefCell<HashMap<*mut c_void, Vec<Rc<TargetState>>>> = RefCell::new(HashMap::new());
    static SOURCES: RefCell<HashMap<*mut c_void, Vec<Rc<SourceState>>>> = RefCell::new(HashMap::new());
}

fn format(name: &str) -> u16 {
    unsafe { RegisterClipboardFormatW(&HSTRING::from(name)) as u16 }
}
fn png_format() -> u16 {
    format("PNG")
}
fn jpeg_format() -> u16 {
    format("JFIF")
}
fn file_descriptor_format() -> u16 {
    format("FileGroupDescriptorW")
}
fn file_contents_format() -> u16 {
    format("FileContents")
}

fn native_operations(value: DragOperations) -> DROPEFFECT {
    let mut result = DROPEFFECT_NONE;
    if value.contains(DragOperations::COPY) {
        result |= DROPEFFECT_COPY;
    }
    if value.contains(DragOperations::MOVE) {
        result |= DROPEFFECT_MOVE;
    }
    if value.contains(DragOperations::LINK) {
        result |= DROPEFFECT_LINK;
    }
    result
}

fn operations(value: DROPEFFECT) -> DragOperations {
    let mut result = DragOperations::NONE;
    if value.contains(DROPEFFECT_COPY) {
        result |= DragOperations::COPY;
    }
    if value.contains(DROPEFFECT_MOVE) {
        result |= DragOperations::MOVE;
    }
    if value.contains(DROPEFFECT_LINK) {
        result |= DragOperations::LINK;
    }
    result
}

fn native_operation(value: Option<DragOperation>) -> DROPEFFECT {
    match value {
        Some(DragOperation::Copy) => DROPEFFECT_COPY,
        Some(DragOperation::Move) => DROPEFFECT_MOVE,
        Some(DragOperation::Link) => DROPEFFECT_LINK,
        None => DROPEFFECT_NONE,
    }
}
fn operation(value: DROPEFFECT) -> Option<DragOperation> {
    if value.contains(DROPEFFECT_MOVE) {
        Some(DragOperation::Move)
    } else if value.contains(DROPEFFECT_COPY) {
        Some(DragOperation::Copy)
    } else if value.contains(DROPEFFECT_LINK) {
        Some(DragOperation::Link)
    } else {
        None
    }
}

fn modifiers(keys: MODIFIERKEYS_FLAGS) -> DragModifiers {
    let mut result = DragModifiers::NONE;
    if keys.contains(MK_CONTROL) {
        result |= DragModifiers::PRIMARY;
    }
    if keys.contains(MK_SHIFT) {
        result |= DragModifiers::SHIFT;
    }
    // OLE's key-state mask uses bit 0x20 for Alt.
    if keys.0 & 0x20 != 0 {
        result |= DragModifiers::ALT;
    }
    result
}

fn format_etc(kind: u16, index: i32) -> FORMATETC {
    FORMATETC {
        cfFormat: kind,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: index,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

fn has_format(data: &IDataObject, kind: u16, index: i32) -> bool {
    unsafe { data.QueryGetData(&format_etc(kind, index)).is_ok() }
}

fn available_types(data: &IDataObject) -> DragDataTypes {
    let mut result = DragDataTypes::NONE;
    if has_format(data, CF_HDROP.0, -1) {
        result |= DragDataTypes::FILES;
    }
    if has_format(data, png_format(), -1)
        || has_format(data, jpeg_format(), -1)
        || has_format(data, CF_DIBV5.0, -1)
        || has_format(data, CF_DIB.0, -1)
    {
        result |= DragDataTypes::IMAGE;
    }
    if has_format(data, CF_UNICODETEXT.0, -1) {
        result |= DragDataTypes::TEXT;
    }
    result
}

struct Medium(STGMEDIUM);
impl Drop for Medium {
    fn drop(&mut self) {
        unsafe { ReleaseStgMedium(&mut self.0) }
    }
}

fn global_bytes(data: &IDataObject, kind: u16) -> Result<Vec<u8>, DragReadError> {
    let medium = Medium(
        unsafe { data.GetData(&format_etc(kind, -1)) }
            .map_err(|e| DragReadError::Backend(e.to_string()))?,
    );
    if medium.0.tymed != TYMED_HGLOBAL.0 as u32 {
        return Err(DragReadError::InvalidData(
            "OLE data is not backed by global memory".into(),
        ));
    }
    let handle = unsafe { medium.0.u.hGlobal };
    let size = unsafe { GlobalSize(handle) };
    let pointer = unsafe { GlobalLock(handle) };
    if pointer.is_null() {
        return Err(DragReadError::InvalidData("unable to lock OLE data".into()));
    }
    let bytes = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), size).to_vec() };
    unsafe {
        let _ = GlobalUnlock(handle);
    }
    Ok(bytes)
}

fn read_text(data: &IDataObject) -> Result<String, DragReadError> {
    let bytes = global_bytes(data, CF_UNICODETEXT.0)?;
    let words =
        unsafe { std::slice::from_raw_parts(bytes.as_ptr().cast::<u16>(), bytes.len() / 2) };
    let end = words
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(words.len());
    String::from_utf16(&words[..end]).map_err(|e| DragReadError::InvalidData(e.to_string()))
}

fn read_files(data: &IDataObject) -> Result<Vec<PathBuf>, DragReadError> {
    let medium = Medium(
        unsafe { data.GetData(&format_etc(CF_HDROP.0, -1)) }
            .map_err(|e| DragReadError::Backend(e.to_string()))?,
    );
    let drop = HDROP(unsafe { medium.0.u.hGlobal }.0);
    let count = unsafe { DragQueryFileW(drop, u32::MAX, None) };
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let length = unsafe { DragQueryFileW(drop, index, None) };
        let mut buffer = vec![0u16; length as usize + 1];
        unsafe {
            DragQueryFileW(drop, index, Some(&mut buffer));
        }
        paths.push(PathBuf::from(String::from_utf16_lossy(
            &buffer[..length as usize],
        )));
    }
    if paths.is_empty() {
        Err(DragReadError::Unavailable(DragDataType::Files))
    } else {
        Ok(paths)
    }
}

fn read_image(data: &IDataObject) -> Result<DragImage, DragReadError> {
    for (kind, media, name) in [
        (png_format(), "image/png", "image.png"),
        (jpeg_format(), "image/jpeg", "image.jpg"),
    ] {
        if has_format(data, kind, -1) {
            return global_bytes(data, kind).map(|bytes| DragImage::new(bytes, media, name));
        }
    }
    for kind in [CF_DIBV5.0, CF_DIB.0] {
        if has_format(data, kind, -1) {
            let dib = global_bytes(data, kind)?;
            if dib.len() < 40 {
                return Err(DragReadError::InvalidData("DIB header is truncated".into()));
            }
            let header = u32::from_le_bytes(dib[0..4].try_into().unwrap()) as usize;
            let bits = if header >= 124 {
                header
            } else {
                let bpp = u16::from_le_bytes(dib[14..16].try_into().unwrap()) as usize;
                let colors = u32::from_le_bytes(dib[32..36].try_into().unwrap()) as usize;
                header
                    + (if colors > 0 {
                        colors * 4
                    } else if bpp <= 8 {
                        (1usize << bpp) * 4
                    } else {
                        0
                    })
            };
            let mut bmp = Vec::with_capacity(dib.len() + 14);
            bmp.extend_from_slice(b"BM");
            bmp.extend_from_slice(&((dib.len() + 14) as u32).to_le_bytes());
            bmp.extend_from_slice(&[0; 4]);
            bmp.extend_from_slice(&((bits + 14) as u32).to_le_bytes());
            bmp.extend_from_slice(&dib);
            return Ok(DragImage::new(bmp, "image/bmp", "image.bmp"));
        }
    }
    Err(DragReadError::Unavailable(DragDataType::Image))
}

fn reader(data: IDataObject, types: DragDataTypes) -> DropDataReader {
    DropDataReader::new(DropDataProvider {
        available_types: types,
        read_files: callback!([data] |done: DragFilesCallback| done(read_files(&data))),
        read_image: callback!([data] |done: DragImageCallback| done(read_image(&data))),
        read_text: callback!([data] |done: DragTextCallback| done(read_text(&data))),
    })
}

struct TargetState {
    hwnd: HWND,
    enabled: PropValue<bool>,
    accepted: PropValue<DragDataTypes>,
    default_operation: PropValue<DragOperation>,
    on_enter: PropValue<Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>>,
    on_over: PropValue<Option<Shared<dyn Fn(&DragOffer) -> Option<DragOperation>>>>,
    on_leave: PropValue<Option<Shared<dyn Fn()>>>,
    on_drop: PropValue<Shared<dyn Fn(DropEvent)>>,
    current: RefCell<Option<(IDataObject, DragDataTypes, DragOperation)>>,
}

fn active_target(hwnd: HWND) -> Option<Rc<TargetState>> {
    TARGETS.with_borrow(|all| all.get(&hwnd.0).and_then(|v| v.last()).cloned())
}

fn offer(
    state: &TargetState,
    data: &IDataObject,
    keys: MODIFIERKEYS_FLAGS,
    point: &POINTL,
    allowed: DROPEFFECT,
) -> DragOffer {
    let mut client = POINT {
        x: point.x,
        y: point.y,
    };
    unsafe {
        let _ = ScreenToClient(state.hwnd, &mut client);
    }
    let scale = unsafe { GetDpiForWindow(state.hwnd) } as f64 / 96.0;
    DragOffer {
        available_types: available_types(data).intersection(state.accepted.get()),
        allowed_operations: operations(allowed),
        position: nestix_native_core::dpi::LogicalPosition::new(
            client.x as f64 / scale,
            client.y as f64 / scale,
        ),
        modifiers: modifiers(keys),
    }
}

fn decide(state: &TargetState, offer: &DragOffer, entering: bool) -> Option<DragOperation> {
    if !state.enabled.get() || offer.available_types.is_empty() {
        return None;
    }
    let callback = if entering {
        state.on_enter.get()
    } else {
        state.on_over.get()
    };
    callback
        .as_ref()
        .and_then(|f| f(offer))
        .or_else(|| callback.is_none().then(|| state.default_operation.get()))
        .filter(|op| offer.allowed_operations.contains_operation(*op))
}

#[implement(IDropTarget)]
struct NativeDropTarget {
    hwnd: HWND,
}

impl IDropTarget_Impl for NativeDropTarget_Impl {
    fn DragEnter(
        &self,
        data: Ref<IDataObject>,
        keys: MODIFIERKEYS_FLAGS,
        point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let Some(state) = active_target(self.hwnd) else {
            unsafe { *effect = DROPEFFECT_NONE };
            return Ok(());
        };
        let data = data.as_ref().ok_or_else(|| Error::from(E_POINTER))?.clone();
        let allowed = unsafe { *effect };
        let value = offer(&state, &data, keys, point, allowed);
        let choice = decide(&state, &value, true);
        state
            .current
            .replace(choice.map(|op| (data, value.available_types, op)));
        unsafe {
            *effect = native_operation(choice);
        }
        Ok(())
    }
    fn DragOver(
        &self,
        keys: MODIFIERKEYS_FLAGS,
        point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let Some(state) = active_target(self.hwnd) else {
            unsafe { *effect = DROPEFFECT_NONE };
            return Ok(());
        };
        let Some((data, _, _)) = state.current.borrow().clone() else {
            unsafe { *effect = DROPEFFECT_NONE };
            return Ok(());
        };
        let value = offer(&state, &data, keys, point, unsafe { *effect });
        let choice = decide(&state, &value, false);
        state
            .current
            .replace(choice.map(|op| (data, value.available_types, op)));
        unsafe {
            *effect = native_operation(choice);
        }
        Ok(())
    }
    fn DragLeave(&self) -> windows::core::Result<()> {
        if let Some(state) = active_target(self.hwnd) {
            state.current.replace(None);
            if let Some(f) = state.on_leave.get() {
                f();
            }
        }
        Ok(())
    }
    fn Drop(
        &self,
        data: Ref<IDataObject>,
        keys: MODIFIERKEYS_FLAGS,
        point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let Some(state) = active_target(self.hwnd) else {
            unsafe { *effect = DROPEFFECT_NONE };
            return Ok(());
        };
        let data = data.as_ref().ok_or_else(|| Error::from(E_POINTER))?.clone();
        let allowed = unsafe { *effect };
        let value = offer(&state, &data, keys, point, allowed);
        let choice = state
            .current
            .borrow()
            .as_ref()
            .map(|v| v.2)
            .filter(|op| value.allowed_operations.contains_operation(*op));
        state.current.replace(None);
        unsafe {
            *effect = native_operation(choice);
        }
        if let Some(op) = choice {
            (state.on_drop.get())(DropEvent {
                operation: op,
                position: value.position,
                modifiers: value.modifiers,
                data: reader(data, value.available_types),
            });
        }
        Ok(())
    }
}

struct TargetRegistration {
    hwnd: HWND,
    state: Rc<TargetState>,
}
impl Drop for TargetRegistration {
    fn drop(&mut self) {
        let last = TARGETS.with_borrow_mut(|all| {
            let Some(stack) = all.get_mut(&self.hwnd.0) else {
                return false;
            };
            stack.retain(|v| !Rc::ptr_eq(v, &self.state));
            if stack.is_empty() {
                all.remove(&self.hwnd.0);
                true
            } else {
                false
            }
        });
        if last {
            unsafe {
                let _ = RevokeDragDrop(self.hwnd);
            }
        }
    }
}

fn register_target(state: Rc<TargetState>) -> windows::core::Result<TargetRegistration> {
    let first = TARGETS.with_borrow_mut(|all| {
        let stack = all.entry(state.hwnd.0).or_default();
        let first = stack.is_empty();
        stack.push(state.clone());
        first
    });
    if first {
        let target: IDropTarget = NativeDropTarget { hwnd: state.hwnd }.into();
        if let Err(error) = unsafe { RegisterDragDrop(state.hwnd, &target) } {
            TARGETS.with_borrow_mut(|all| {
                all.remove(&state.hwnd.0);
            });
            return Err(error);
        }
    }
    Ok(TargetRegistration {
        hwnd: state.hwnd,
        state,
    })
}

#[component]
pub fn DropTarget(props: &DropTargetProps, element: &Element) -> Element {
    let registration = Rc::new(RefCell::new(None::<TargetRegistration>));
    scoped_effect!(
        element,
        [
            registration,
            props.children,
            props.enabled,
            props.accepted_types,
            props.default_operation,
            props.on_enter,
            props.on_over,
            props.on_leave,
            props.on_drop
        ] || {
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
                        let Some(hwnd) = handle.downcast_ref::<HWND>() else {
                            return;
                        };
                        let state = Rc::new(TargetState {
                            hwnd: *hwnd,
                            enabled: enabled.clone(),
                            accepted: accepted_types.clone(),
                            default_operation: default_operation.clone(),
                            on_enter: on_enter.clone(),
                            on_over: on_over.clone(),
                            on_leave: on_leave.clone(),
                            on_drop: on_drop.clone(),
                            current: RefCell::new(None),
                        });
                        if let Ok(value) = register_target(state) {
                            registration.borrow_mut().replace(value);
                        }
                    }
            ));
        }
    );
    element.on_unmount(closure!(
        [registration] || {
            registration.borrow_mut().take();
        }
    ));
    layout! { $(props.children.get()) }
}

fn hglobal(bytes: &[u8]) -> windows::core::Result<STGMEDIUM> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len()) }?;
    let pointer = unsafe { GlobalLock(handle) };
    if pointer.is_null() {
        return Err(Error::new(E_POINTER, "unable to lock drag data memory"));
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast(), bytes.len());
        let _ = GlobalUnlock(handle);
    }
    Ok(STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: handle },
        pUnkForRelease: ManuallyDrop::new(None),
    })
}

fn unicode_bytes(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}
fn hdrop_bytes(paths: &[PathBuf]) -> Vec<u8> {
    let mut result = vec![0u8; 20];
    result[0..4].copy_from_slice(&20u32.to_le_bytes());
    result[16..20].copy_from_slice(&1u32.to_le_bytes());
    for path in paths {
        result.extend(unicode_bytes(&path.to_string_lossy()));
    }
    result.extend_from_slice(&[0, 0]);
    result
}

#[repr(C)]
struct FileDescriptorW {
    flags: u32,
    clsid: [u8; 16],
    sizel: [i32; 2],
    pointl: [i32; 2],
    attributes: u32,
    creation: u64,
    access: u64,
    write: u64,
    high: u32,
    low: u32,
    name: [u16; 260],
}
fn virtual_items(content: &DragContent) -> Vec<(String, Arc<[u8]>)> {
    let mut items = Vec::new();
    if let Some(image) = content.image() {
        items.push((image.suggested_name.clone(), image.bytes.clone()));
    }
    if let Some(text) = content.text() {
        items.push(("nestix.txt".into(), Arc::from(text.as_bytes())));
    }
    items
}
fn descriptors(content: &DragContent) -> Vec<u8> {
    let items = virtual_items(content);
    let mut out = Vec::with_capacity(4 + items.len() * size_of::<FileDescriptorW>());
    out.extend_from_slice(&(items.len() as u32).to_le_bytes());
    for (name, bytes) in items {
        let mut value = FileDescriptorW {
            flags: FD_ATTRIBUTES | FD_FILESIZE,
            clsid: [0; 16],
            sizel: [0; 2],
            pointl: [0; 2],
            attributes: FILE_ATTRIBUTE_NORMAL,
            creation: 0,
            access: 0,
            write: 0,
            high: (bytes.len() as u64 >> 32) as u32,
            low: bytes.len() as u32,
            name: [0; 260],
        };
        for (to, from) in value.name.iter_mut().zip(name.encode_utf16()) {
            *to = from;
        }
        out.extend_from_slice(unsafe {
            std::slice::from_raw_parts(
                (&value as *const FileDescriptorW).cast::<u8>(),
                size_of::<FileDescriptorW>(),
            )
        });
    }
    out
}

#[implement(IDataObject)]
struct NativeDataObject {
    content: DragContent,
}
impl NativeDataObject {
    fn formats(&self) -> Vec<FORMATETC> {
        let mut v = Vec::new();
        if self.content.files().is_some_and(|x| !x.is_empty()) {
            v.push(format_etc(CF_HDROP.0, -1));
        }
        if self.content.text().is_some() {
            v.push(format_etc(CF_UNICODETEXT.0, -1));
        }
        if let Some(image) = self.content.image() {
            v.push(format_etc(
                if image.media_type.eq_ignore_ascii_case("image/png") {
                    png_format()
                } else {
                    jpeg_format()
                },
                -1,
            ));
        }
        let virtuals = virtual_items(&self.content);
        if !virtuals.is_empty() {
            v.push(format_etc(file_descriptor_format(), -1));
            for i in 0..virtuals.len() {
                v.push(format_etc(file_contents_format(), i as i32));
            }
        }
        v
    }
}
impl IDataObject_Impl for NativeDataObject_Impl {
    fn GetData(&self, input: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
        if input.is_null() {
            return Err(Error::from(E_POINTER));
        }
        let f = unsafe { &*input };
        if f.cfFormat == CF_HDROP.0 {
            return hglobal(&hdrop_bytes(self.content.files().unwrap_or_default()));
        }
        if f.cfFormat == CF_UNICODETEXT.0 {
            return hglobal(&unicode_bytes(self.content.text().unwrap_or_default()));
        }
        if f.cfFormat == file_descriptor_format() {
            return hglobal(&descriptors(&self.content));
        }
        if f.cfFormat == file_contents_format() {
            return virtual_items(&self.content)
                .get(f.lindex as usize)
                .map(|x| hglobal(&x.1))
                .unwrap_or_else(|| Err(Error::from(DV_E_FORMATETC)));
        }
        if let Some(image) = self.content.image() {
            if f.cfFormat == png_format() || f.cfFormat == jpeg_format() {
                return hglobal(&image.bytes);
            }
        }
        Err(Error::from(DV_E_FORMATETC))
    }
    fn GetDataHere(&self, _: *const FORMATETC, _: *mut STGMEDIUM) -> windows::core::Result<()> {
        Err(Error::from(E_NOTIMPL))
    }
    fn QueryGetData(&self, input: *const FORMATETC) -> HRESULT {
        if input.is_null() {
            return E_POINTER;
        }
        let f = unsafe { &*input };
        if self.formats().iter().any(|x| {
            x.cfFormat == f.cfFormat
                && (x.lindex == f.lindex || x.lindex == -1)
                && f.tymed & TYMED_HGLOBAL.0 as u32 != 0
        }) {
            HRESULT(0)
        } else {
            DV_E_FORMATETC
        }
    }
    fn GetCanonicalFormatEtc(&self, _: *const FORMATETC, output: *mut FORMATETC) -> HRESULT {
        if !output.is_null() {
            unsafe {
                (*output).ptd = std::ptr::null_mut();
            }
        }
        windows::Win32::Foundation::DATA_S_SAMEFORMATETC
    }
    fn SetData(
        &self,
        _: *const FORMATETC,
        _: *const STGMEDIUM,
        _: windows::core::BOOL,
    ) -> windows::core::Result<()> {
        Err(Error::from(E_NOTIMPL))
    }
    fn EnumFormatEtc(&self, direction: u32) -> windows::core::Result<IEnumFORMATETC> {
        if direction != 1 {
            return Err(Error::from(E_NOTIMPL));
        }
        unsafe { SHCreateStdEnumFmtEtc(&self.formats()) }
    }
    fn DAdvise(
        &self,
        _: *const FORMATETC,
        _: u32,
        _: Ref<IAdviseSink>,
    ) -> windows::core::Result<u32> {
        Err(Error::from(E_NOTIMPL))
    }
    fn DUnadvise(&self, _: u32) -> windows::core::Result<()> {
        Err(Error::from(E_NOTIMPL))
    }
    fn EnumDAdvise(&self) -> windows::core::Result<IEnumSTATDATA> {
        Err(Error::from(E_NOTIMPL))
    }
}

#[implement(IDropSource)]
struct NativeDropSource;
impl IDropSource_Impl for NativeDropSource_Impl {
    fn QueryContinueDrag(&self, escape: windows::core::BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape.as_bool() {
            DRAGDROP_S_CANCEL
        } else if !keys.contains(MK_LBUTTON) {
            DRAGDROP_S_DROP
        } else {
            HRESULT(0)
        }
    }
    fn GiveFeedback(&self, _: DROPEFFECT) -> HRESULT {
        windows::Win32::Foundation::DRAGDROP_S_USEDEFAULTCURSORS
    }
}

struct SourceState {
    hwnd: HWND,
    content: PropValue<DragContent>,
    enabled: PropValue<bool>,
    allowed: PropValue<DragOperations>,
    on_started: PropValue<Option<Shared<dyn Fn()>>>,
    on_completed: PropValue<Option<Shared<dyn Fn(DragSourceOutcome)>>>,
    on_error: PropValue<Option<Shared<dyn Fn(DragSourceError)>>>,
    origin: Cell<Option<POINT>>,
}
fn active_source(hwnd: HWND) -> Option<Rc<SourceState>> {
    SOURCES.with_borrow(|all| all.get(&hwnd.0).and_then(|v| v.last()).cloned())
}
unsafe extern "system" fn source_subclass(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if let Some(state) = active_source(hwnd) {
        let point = POINT {
            x: (l.0 as i16) as i32,
            y: ((l.0 >> 16) as i16) as i32,
        };
        if msg == WM_LBUTTONDOWN {
            state.origin.set(Some(point));
        } else if msg == WM_LBUTTONUP {
            state.origin.set(None);
        } else if msg == WM_MOUSEMOVE && w.0 & 1 != 0 {
            if let Some(start) = state.origin.get() {
                let dx = (point.x - start.x).abs();
                let dy = (point.y - start.y).abs();
                if dx >= unsafe { GetSystemMetrics(SM_CXDRAG) }
                    || dy >= unsafe { GetSystemMetrics(SM_CYDRAG) }
                {
                    state.origin.set(None);
                    begin_drag(&state);
                    return LRESULT(0);
                }
            }
        }
    }
    unsafe { DefSubclassProc(hwnd, msg, w, l) }
}
fn begin_drag(state: &SourceState) {
    if !state.enabled.get() {
        return;
    }
    let content = state.content.get();
    if content.is_empty() {
        if let Some(f) = state.on_error.get() {
            f(DragSourceError::EmptyContent)
        }
        return;
    }
    if let Some(f) = state.on_started.get() {
        f()
    }
    let data: IDataObject = NativeDataObject { content }.into();
    let source: IDropSource = NativeDropSource.into();
    let mut effect = DROPEFFECT_NONE;
    let hr = unsafe {
        DoDragDrop(
            &data,
            &source,
            native_operations(state.allowed.get()),
            &mut effect,
        )
    };
    if hr == DRAGDROP_S_DROP {
        if let Some(f) = state.on_completed.get() {
            f(operation(effect)
                .map(DragSourceOutcome::Dropped)
                .unwrap_or(DragSourceOutcome::Cancelled))
        }
    } else if hr == DRAGDROP_S_CANCEL {
        if let Some(f) = state.on_completed.get() {
            f(DragSourceOutcome::Cancelled)
        }
    } else if let Some(f) = state.on_error.get() {
        f(DragSourceError::Backend(format!(
            "OLE drag failed ({hr:?})"
        )))
    }
}
struct SourceRegistration {
    hwnd: HWND,
    state: Rc<SourceState>,
}
impl Drop for SourceRegistration {
    fn drop(&mut self) {
        let last = SOURCES.with_borrow_mut(|all| {
            let Some(v) = all.get_mut(&self.hwnd.0) else {
                return false;
            };
            v.retain(|x| !Rc::ptr_eq(x, &self.state));
            if v.is_empty() {
                all.remove(&self.hwnd.0);
                true
            } else {
                false
            }
        });
        if last {
            unsafe {
                let _ = RemoveWindowSubclass(self.hwnd, Some(source_subclass), SOURCE_SUBCLASS_ID);
            }
        }
    }
}
fn register_source(state: Rc<SourceState>) -> Option<SourceRegistration> {
    let first = SOURCES.with_borrow_mut(|all| {
        let v = all.entry(state.hwnd.0).or_default();
        let first = v.is_empty();
        v.push(state.clone());
        first
    });
    if first
        && !unsafe { SetWindowSubclass(state.hwnd, Some(source_subclass), SOURCE_SUBCLASS_ID, 0) }
            .as_bool()
    {
        SOURCES.with_borrow_mut(|all| {
            all.remove(&state.hwnd.0);
        });
        return None;
    }
    Some(SourceRegistration {
        hwnd: state.hwnd,
        state,
    })
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
                        let Some(hwnd) = handle.downcast_ref::<HWND>() else {
                            return;
                        };
                        let state = Rc::new(SourceState {
                            hwnd: *hwnd,
                            content: content.clone(),
                            enabled: enabled.clone(),
                            allowed: allowed_operations.clone(),
                            on_started: on_started.clone(),
                            on_completed: on_completed.clone(),
                            on_error: on_error.clone(),
                            origin: Cell::new(None),
                        });
                        if let Some(value) = register_source(state) {
                            registration.borrow_mut().replace(value);
                        }
                    }
            ));
        }
    );
    element.on_unmount(closure!(
        [registration] || {
            registration.borrow_mut().take();
        }
    ));
    layout! {$(props.children.get())}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operation_mapping_round_trips() {
        for op in [
            DragOperation::Copy,
            DragOperation::Move,
            DragOperation::Link,
        ] {
            assert_eq!(operation(native_operation(Some(op))), Some(op));
        }
    }
    #[test]
    fn hdrop_is_double_nul_terminated() {
        let bytes = hdrop_bytes(&[PathBuf::from(r"C:\a.txt")]);
        assert_eq!(&bytes[bytes.len() - 4..], &[0, 0, 0, 0]);
    }
}
