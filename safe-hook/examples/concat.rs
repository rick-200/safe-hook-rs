use safe_hook::Hook;
use safe_hook_macros::hookable;
use std::sync::Arc;

#[hookable("concat")]
fn concat<'a>(left: &'a str, right: &'a str) -> String {
    format!("{}-{}", left, right)
}

struct ConcatHook {
    mid: String,
}

impl Hook for ConcatHook {
    type Args<'b> = (&'b str, &'b str);
    type Result = String;

    fn call<'a>(
        &'a self,
        args: Self::Args<'a>,
        next: &dyn for<'c> Fn(Self::Args<'c>) -> Self::Result,
    ) -> Self::Result {
        let (left, right) = args;
        let lest_new = format!("{}-{}", left, self.mid);
        next((&lest_new, right))
    }
}

fn main() {
    let hookable_metadata = safe_hook::lookup_hookable("concat").unwrap();
    assert_eq!(concat("abc", "def"), "abc-def");
    let hook = Arc::new(ConcatHook {
        mid: "hook".to_string(),
    });
    hookable_metadata.add_hook(hook).unwrap();
    assert_eq!(concat("abc", "def"), "abc-hook-def");
}
