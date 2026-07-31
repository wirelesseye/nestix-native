use nestix::{ContextProvider, layout, mount_root, unmount_root};
use nestix_native_appkit::{Button, Sidebar};
use nestix_native_core::NativeVisualMount;

#[test]
fn direct_visual_component_stops_at_blocked_boundary() {
    let tree = layout! {
        ContextProvider<NativeVisualMount>(NativeVisualMount::blocked("test boundary")) {
            Button(.title = "Blocked")
        }
    };

    mount_root(&tree);
    unmount_root().unwrap();
}

#[test]
fn direct_visual_component_stops_at_foreign_visual_tree() {
    let tree = layout! {
        ContextProvider<NativeVisualMount>(NativeVisualMount::allowed("another-backend")) {
            Button(.title = "Foreign")
        }
    };

    mount_root(&tree);
    unmount_root().unwrap();
}

#[test]
fn sidebar_stops_at_blocked_boundary_before_requiring_a_window() {
    let tree = layout! {
        ContextProvider<NativeVisualMount>(NativeVisualMount::blocked("test boundary")) {
            Sidebar()
        }
    };

    mount_root(&tree);
    unmount_root().unwrap();
}

#[test]
fn sidebar_stops_at_foreign_visual_tree_before_requiring_a_window() {
    let tree = layout! {
        ContextProvider<NativeVisualMount>(NativeVisualMount::allowed("another-backend")) {
            Sidebar()
        }
    };

    mount_root(&tree);
    unmount_root().unwrap();
}
