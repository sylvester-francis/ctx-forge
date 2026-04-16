use iocraft::prelude::*;

#[component]
pub fn BundleSummary(hooks: &mut Hooks) -> impl Into<AnyElement<'static>> {
    let _ = hooks;
    element! { Text(content: "[bundle stub]") }
}
