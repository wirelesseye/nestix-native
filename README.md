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

The facade crate enables both backend features by default, but only the backend
for the current compilation target is used. Builds for unsupported platforms, or
builds where the relevant platform feature is disabled, must provide their own
backend context or will fail at runtime when the default backend is requested.

### Alternative backend(s)

[`nestix-native-winui`](https://github.com/wirelesseye/nestix-native-winui) is
an experimental Windows backend exploring WinUI as an alternative to the Win32
backend in this repository.

## Examples

The workspace includes two examples:

- `examples/basic` shows a counter window with text, buttons, callbacks, state,
  and simple layout.
- `examples/tabs` shows tabs, editable input, dynamic lists, conditional
  rendering, and reactive styles.

They are intended as reference material for the current shape of the API rather
than as comprehensive documentation.
