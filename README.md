# Nestix Native

Nestix Native is a cross-platform GUI library for building user
interfaces with the native UI toolkits available on each operating system. It is
powered by [`Nestix`](https://github.com/wirelesseye/nestix), a declarative layout and state management library for
Rust.

> [!WARNING]  
> This library is still in early stages of development, APIs can break at any time.

## Key Features

- Native UI backends with a shared component API.
- Reactive state, computed values, callbacks, and conditional rendering through
  Nestix.
- A small common widget set that can be implemented consistently across
  platforms.
- CSS-like styling for layout and simple visual properties.
- Backend crates that can evolve independently while sharing core props, style
  parsing, and layout concepts.

## Platform support

Nestix Native currently provides built-in backend crates for:

| Platform | Backend crate | Default feature |
| --- | --- | --- |
| macOS | `nestix-native-appkit` | `appkit` |
| Windows | `nestix-native-win32` | `win32` |
| Linux | `nestix-native-gtk4` | `gtk4` |
| Browser (`wasm32`) | [`nestix-native-dom`](nestix-native-dom/README.md) | `dom` |

The facade crate enables all backend features by default, but only the backend
for the current compilation target is used. Builds for unsupported platforms, or
builds where the relevant platform feature is disabled, must provide their own
backend context or will fail at runtime when the default backend is requested.

Browser setup and backend-specific APIs are documented in the
[`nestix-native-dom` README](nestix-native-dom/README.md). Desktop-only window
options are grouped under `Window(.desktop(...))`.

## Managed DOM surfaces

On macOS, enabling both the `appkit` and `dom` features provides `DomSurface`.
It is laid out as one native view while its descendants are rendered by the DOM
backend inside a managed `WKWebView` document:

```rust
layout! {
    FlexView {
        Button(.title = "Native button")
        DomSurface(
            .view(.height = 160),
            .transparent = false,
        ) {
            Button(.title = "DOM button")
        }
    }
}
```

The native and DOM controls remain in one Nestix tree, so they share signals,
computed values, callbacks, styling, and lifecycle. The embedded backend
currently supports `Button`, `Input`, `Text`, and `FlexView`. `WebView` remains
the URL-loading component and does not accept a Nestix subtree. See
`examples/dom-surface` for a complete mixed native/DOM application.
`DomSurface` has a transparent background by default. Set `transparent` to
`false` to use WebKit's normal opaque background.

### Alternative backend(s)

[`nestix-native-winui`](https://github.com/wirelesseye/nestix-native-winui) is
an experimental Windows backend exploring WinUI as an alternative to the Win32
backend in this repository.

## Examples

The workspace includes these examples:

- `examples/basic` shows a counter window with text, buttons, callbacks, state,
  and simple layout.
- `examples/context-menu` demonstrates context menu commands, checkboxes, radio
  items, submenus, and programmatic presentation.
- `examples/drag-drop` demonstrates a drag source and lazy drop
  target for files, encoded images, and UTF-8 text.
- `examples/dom-basic` demonstrates selector-based browser mounting, DOM
  controls, reactive state, and shared Nestix Native styling.
- `examples/dom-surface` demonstrates native AppKit controls and managed DOM
  elements sharing the same signals and callbacks in one window.
- `examples/file-picker` demonstrates open, multi-file, save, and folder picker
  requests through a window-bound controller.
- `examples/menu-bar` demonstrates application-wide and window-specific menu
  bars.
- `examples/tray-icon` demonstrates primary and secondary tray activation,
  explicit menu presentation, reactive visibility, and shared menu components.
- `examples/tabs` shows tabs, editable input, dynamic lists, conditional
  rendering, and reactive styles.
- `examples/web-view` demonstrates reactive URL navigation with `WKWebView` on
  macOS and an `iframe` in browser WebAssembly builds.

They are intended as reference material for the current shape of the API rather
than as comprehensive documentation.
