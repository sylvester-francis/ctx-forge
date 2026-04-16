use iocraft::prelude::*;

#[component]
pub fn FileTree(hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    let _ = hooks;
    element! { Text(content: "[tree stub]") }
}
