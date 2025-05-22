use safe_hook::{Hook, lookup_hookable};
use safe_hook_macros::hookable;
use std::sync::Arc;
#[hookable("add")]
fn add(left: i64, right: i64) -> i64 {
    left + right
}
#[derive(Debug)]
struct HookAdd {
    x: i64,
}
impl Hook for HookAdd {
    type Args<'a> = (i64, i64);
    type Result = i64;
    fn call(&self, args: (i64, i64), next: &dyn Fn((i64, i64)) -> i64) -> i64 {
        next(args) + self.x
    }
}
fn main() {
    let hook = Arc::new(HookAdd { x: 1 });
    assert_eq!(add(1, 2), 3);
    lookup_hookable("add").unwrap().add_hook(hook).unwrap();
    assert_eq!(add(1, 2), 4);
}
