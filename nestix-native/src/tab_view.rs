pub use nestix_native_core::{TabViewItemProps, TabViewProps};

delegate!(
    /// Displays child pages in a tabbed container.
    pub TabView(TabViewProps) => create_tab_view,
    /// Defines one labelled page within a [`TabView`].
    pub TabViewItem(TabViewItemProps) => create_tab_view_item,
);
