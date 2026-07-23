pub use nestix_native_core::{
    DragContent, DragDataType, DragDataTypes, DragImage, DragModifiers, DragOffer, DragOperation,
    DragOperations, DragReadError, DragSourceError, DragSourceOutcome, DragSourceProps,
    DropDataReader, DropEvent, DropTargetProps,
};

delegate!(
    /// Makes its child a native drag source for files, images, or text.
    pub DragSource(DragSourceProps) => create_drag_source,
    /// Makes its child accept compatible native drag-and-drop data.
    pub DropTarget(DropTargetProps) => create_drop_target,
);
