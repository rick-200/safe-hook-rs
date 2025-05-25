# Safe-Hook
Safe-Hook is an inline hook library for Rust.
It provides a simple and safe way to create hooks in your Rust applications,
allowing you to modify the behavior of functions at runtime.

The design principle of Safe-Hook is safety and simplicity.

## Features
- **Inline Hooking**: Safe-Hook allows you to hook into functions at runtime,
  enabling you to modify their behavior.
- **Safe and Simple**: The library is designed to be safe and easy to use,
  it checks types of parameters and return values at runtime to ensure safety.
- **Full Dynamic**: Safe-Hook is fully dynamic,
  allowing you to add and remove hooks at runtime without any restrictions.
- **Cross-Platform**: Safe-Hook is designed to work on multiple platforms,
  it theoretically supports all platforms that Rust supports.

## Limitations
- **Intrusive**: Needs to annotate target functions manually.
  Which means it's not suitable for hook third-party libraries.


## Usage
More Examples:
- [Hook a function with reference parameters](#hook-a-function-with-reference-parameters)

Simple Usage:
```rust
use std::sync::Arc;
use safe_hook::{lookup_hookable, Hook};
use safe_hook_macros::hookable;

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
    let hook = Arc::new(HookAdd {
        x: 1,
    });
    assert_eq!(add(1, 2), 3);
    lookup_hookable("add").unwrap().add_hook(hook).unwrap();
    assert_eq!(add(1, 2), 4);
}
```

## Performance
Extra overhead:
- No Hook Added: One atomic load and one branch jump,
  which should be very lightweight in most cases.
- Hooks Added: There is a read/write lock (just some atomic operations in most cases),
  some additional function calls via pointers,
  and some copy operations to pack parameters into a tuple.

A sloppy benchmark (uses 12700H) shows that the extra overhead is
about 0.5ns when no hooks are added
(as a comparison, an `add(a,b)` function takes about 0.5ns),
about 14ns when hooks are added,
and that each additional hook results in about 2ns of overhead.

## More Examples
### Hook a function with reference parameters
To hook a function containing referenced parameters,
it must be guaranteed that all referenced parameters
have the same lifecycle, this is a current implementation limitation,
and in fact should be equivalent for the caller,
since the rust compiler should always be able to
convert long lifecycle references to short lifecycle references
(if this is wrong, please let me know).

```rust
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
```