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
        let res = next((&lest_new, right));
        res
    }
}

#[test]
fn test() {
    let hookable_metadata = safe_hook::lookup_hookable("concat").unwrap();
    assert_eq!(concat("abc", "def"), "abc-def");
    let hook1 = Arc::new(ConcatHook {
        mid: "hook1".to_string(),
    });
    hookable_metadata.add_hook(hook1.clone()).unwrap();
    assert_eq!(concat("abc", "def"), "abc-hook1-def");
    let hook2 = Arc::new(ConcatHook {
        mid: "hook2".to_string(),
    });
    hookable_metadata.add_hook(hook2.clone()).unwrap();
    assert_eq!(concat("abc", "def"), "abc-hook2-hook1-def");
    let x = hookable_metadata.remove_hook(hook1.as_ref());
    assert!(x);
    assert_eq!(concat("abc", "def"), "abc-hook2-def");
    let x = hookable_metadata.remove_hook(hook2.as_ref());
    assert!(x);
    assert_eq!(concat("abc", "def"), "abc-def");
    let x = hookable_metadata.remove_hook(hook2.as_ref());
    assert!(!x);
}
