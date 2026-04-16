use iocraft::prelude::*;

#[component]
pub fn PromptInput(hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    let _ = hooks;
    element! { Text(content: "[prompt input stub]") }
}
