use iocraft::prelude::*;

#[component]
pub fn StatusBar(hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    let _ = hooks;
    element! { Text(content: "[status stub]") }
}
