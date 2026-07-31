pub use nestix_native_core::SidebarProps;

delegate!(
    /// Attaches a sidebar pane to the nearest containing window.
    pub Sidebar(SidebarProps) => create_sidebar,
);
