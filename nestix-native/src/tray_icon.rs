pub use nestix_native_core::TrayIconProps;

delegate!(
    /// Adds an application icon to the notification area or status bar.
    pub TrayIcon(TrayIconProps) => create_tray_icon
);
