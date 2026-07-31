pub use nestix_native_core::{NavigationItemProps, SidebarNavigationProps};

delegate!(
    /// Displays navigation items using the platform's sidebar navigation style.
    pub SidebarNavigation(SidebarNavigationProps) => create_sidebar_navigation,
    /// Defines one labelled value within [`SidebarNavigation`].
    pub NavigationItem(NavigationItemProps) => create_navigation_item,
);
