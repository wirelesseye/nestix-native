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

## Collection views

AppKit provides keyed, controlled `ListView`, `TableView`, and `TreeView`
components. The views accept reactive application data and a child factory that
returns a text descriptor for each item:

```rust
ListView<Person>(
    .items = people,
    .key = callback!(|person: &Person| person.id.clone()),
    .value = selected,
    .on_value_change = callback!(|value: &str| { /* update selected */ }),
) |person: Readonly<Person>| {
    ListViewItem(computed!([person] || person.get().name))
}
```

`TableView` maps `TableViewCell` descriptors to stable `TableViewColumn` IDs.
`TreeView` additionally accepts a `child_items` callback and keeps expansion as
native, uncontrolled state. Selection values are the globally unique strings
returned by `key`; double-clicking or pressing Return invokes `on_activate`.
Other backends currently report these components as unsupported.
The same API is re-exported by `nestix-native-appkit`, so AppKit-only
applications can use the collection views without depending on the facade
crate.

## Web content

`WebView` accepts a `WebViewSource`, independently of `DomSurface` and
its native bridge runtime:

```rust
WebView(WebViewSource::url("https://example.com"))

WebView(WebViewSource::html(
    "<!doctype html><body>Application HTML</body>",
))

WebView(
    WebViewSource::resource("web/index.html")
        .with_development_path("assets/web/index.html"),
)
```

Sources are reactive, so a state or computed value containing a
`WebViewSource` can navigate or replace the document after mounting.
On native platforms, resource paths are relative to the packaged application
resource directory; `development_path` is the unpackaged `cargo run` fallback.
Set `.inspectable = true` to let users inspect a web view with Web Inspector on
macOS or WebView2 DevTools on Windows. Developer tools are disabled by default;
applications commonly enable them only in debug builds with
`.inspectable = cfg!(debug_assertions)`.

Pass a `WebViewController` to open the developer tools programmatically:

```rust
let web_view = WebViewController::new();

layout! {
    WebView(
        WebViewSource::url("https://example.com"),
        .inspectable = true,
        .controller = web_view.clone(),
    )
}

web_view.open_dev_tools()?;
```

On macOS, opening Web Inspector programmatically uses private WebKit APIs. It
is available in debug builds; release builds must explicitly enable the
`devtools` Cargo feature and may not be suitable for App Store distribution.

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
It also accepts the same `.inspectable` and `.controller` options as `WebView` for
inspecting the managed document. Inspection is disabled by default.

Applications can provide the surrounding document with `DomTemplate`. Nestix
injects its root, default styles, and bridge script at document start; the
template remains free to load its own stylesheets and scripts. Mark the desired
mount point with `data-nestix-root`, or Nestix will append one to `body`:

```rust
DomSurface(
    .template = DomTemplate::resource("web/index.html")
        .with_development_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/web/index.html",
        )),
) {
    Button(.title = "DOM button")
}
```

The resource path is relative to the packaged application's resource directory.
For `cargo-packager`, map the directory in the example package metadata:

```toml
[package.metadata.packager]
resources = [{ src = "assets/web", target = "web" }]
```

`development_path` is only a fallback for running the unpackaged binary with
`cargo run`. Inline HTML is also available through `DomTemplate::html` and
`DomTemplate::html_with_base_url`.

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
- `examples/materials` demonstrates reactive switching between composited
  materials and their behind-window or within-window sampling sources.
- `examples/tray-icon` demonstrates primary and secondary tray activation,
  explicit menu presentation, reactive visibility, and shared menu components.
- `examples/tabs` shows tabs, editable input, dynamic lists, conditional
  rendering, and reactive styles.
- `examples/web-view` demonstrates reactive URL navigation with `WKWebView` on
  macOS and an `iframe` in browser WebAssembly builds.

They are intended as reference material for the current shape of the API rather
than as comprehensive documentation.
