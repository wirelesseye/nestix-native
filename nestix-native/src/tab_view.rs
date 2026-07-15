pub use nestix_native_core::{TabViewItemProps, TabViewProps};

delegate!(
    pub TabView(TabViewProps) => create_tab_view,
    pub TabViewItem(TabViewItemProps) => create_tab_view_item,
);
