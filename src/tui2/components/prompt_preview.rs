use iocraft::prelude::*;

#[component]
pub fn PromptPreview(hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    let _ = hooks;
    element! { Text(content: "[preview stub]") }
}
