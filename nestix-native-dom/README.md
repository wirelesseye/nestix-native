# Nestix Native DOM

`nestix-native-dom` renders Nestix Native component trees into browser DOM
nodes on `wasm32-unknown-unknown`.

The initial backend supports `Root`, `Window`, `FlexView`, `Text`, `Button`,
and `Input`. It also provides `DomElement` for arbitrary HTML and registered
custom elements.

## Mounting

Browser applications must mount through this backend and supply a selector for
an existing host element:

```rust
let app = layout! { App };
nestix_native_dom::mount_root("#app", &app);
```

The backend does not fall back to `document.body` or an implicit `#root`.
`Window` is rendered as an in-page application surface rather than a popup.

## Styling

Nestix classes participate in Rust-side selector matching but are not copied
to the generated DOM. The backend applies the resolved Nestix Native style as
inline CSS, including layout, flex, font, color, and transition properties.

Backend default selector classes use the `__Component` and
`__dom_Component` forms.

## Arbitrary and custom elements

Use `DomElement` for native HTML elements or registered custom elements such
as `sp-button`:

```rust
DomElement(
    "sp-button",
    .class = "save_action",        // Nestix selector class; not emitted
    .dom_class = "toolbar-button", // Actual HTML class
    .attributes = computed!([disabled] || vec![
        DomAttribute::string("variant", "accent"),
        DomAttribute::boolean("disabled", disabled.get()),
    ]),
    .properties = computed!([value] || vec![
        DomProperty::new("value", JsValue::from_str(&value.get())),
    ]),
    .events = vec![DomEvent::new("click", move |event| {
        // Custom events can be downcast to the appropriate web-sys type.
    })],
    .node_ref = element_ref.clone(),
) {
    Text("Save")
}
```

Attribute and property vectors may be plain or reactive. Boolean attributes
use presence semantics, and entries removed from a reactive vector are removed
from the element. JavaScript properties are assigned with `Reflect.set`.

`DomEvent` listeners are removed during unmount and support `capture`, `once`,
and `passive` options. `DomElementRef` exposes the underlying
`web_sys::Element` while mounted and clears itself during unmount.

The HTML `class` and `style` attributes are reserved. Use `dom_class` for an
emitted class and Nestix styles for presentation. Custom-element definitions
must be loaded before the Nestix tree is mounted.

## Example

See [`examples/dom-basic`](../examples/dom-basic) for a WASM application using
shared native components and a custom DOM element.
