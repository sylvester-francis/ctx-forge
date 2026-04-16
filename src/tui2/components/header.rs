use iocraft::prelude::*;

#[component]
pub fn Header(hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    let _ = hooks;
    element! { Text(content: "[header stub]") }
}
