use safe_hook::{Hook, lookup_hookable};
use safe_hook_macros::hookable;
use std::fmt::Debug;
use std::sync::Arc;

#[hookable("add")]
fn add(left: i64, right: i64) -> i64 {
    left + right
}

#[derive(Debug)]
struct HookAdd {
    left: i64,
    right: i64,
    result: i64,
}

impl Hook for HookAdd {
    type Args<'a> = (i64, i64);
    type Result = i64;
    fn call(&self, args: (i64, i64), next: &dyn Fn((i64, i64)) -> i64) -> i64 {
        println!("hook {:?} called with args: {:?}", self, args);
        let (left, right) = args;
        let res = next((left + self.left, right + self.right));
        res + self.result
    }
}

#[test]
fn test() {
    let hook1 = Arc::new(HookAdd {
        left: 1,
        right: 0,
        result: 0,
    });
    let hook2 = Arc::new(HookAdd {
        left: 0,
        right: 1,
        result: 0,
    });
    let hook3 = Arc::new(HookAdd {
        left: 0,
        right: 0,
        result: 1,
    });
    let add_hookable = lookup_hookable("add").unwrap();
    assert_eq!(add(1, 2), 3);
    add_hookable.add_hook(hook1.clone()).unwrap();
    assert_eq!(add(1, 2), 4);
    add_hookable.add_hook(hook2.clone()).unwrap();
    assert_eq!(add(1, 2), 5);
    add_hookable.add_hook(hook3.clone()).unwrap();
    assert_eq!(add(1, 2), 6);
    add_hookable.remove_hook(hook1.as_ref());
    assert_eq!(add(1, 2), 5);
    add_hookable.remove_hook(hook2.as_ref());
    assert_eq!(add(1, 2), 4);
    add_hookable.remove_hook(hook3.as_ref());
    assert_eq!(add(1, 2), 3);
}
