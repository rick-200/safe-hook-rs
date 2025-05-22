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

## Usage
For more examples, please refer to:
- [add.rs](./safe-hook/tests/add.rs): 
  An example of how to hook a function.
- [concat.rs](./safe-hook/tests/concat.rs): 
  An example of how to hook a function with reference parameters.
```rust
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
    let hook = Arc::new(HookAdd {
        left: 1,
        right: 1,
        result: 1,
    });
    let add_hookable = lookup_hookable("add").unwrap();
    assert_eq!(add(1, 2), 3);
    add_hookable.add_hook(hook1).unwrap();
    assert_eq!(add(1, 2), 6);
}
```



## Limitations
- **Intrusive**: Needs to annotate target functions manually.
  Which means it's not suitable for hook third-party libraries.
- **Limited to Rust**: Safe-Hook is designed specifically for Rust applications.
- **Not Zero-Cost**: The library adds some overhead to the hooked functions,
  which may not be suitable for performance-critical applications.

## Performance
A sloppy benchmark (uses 12700H) shows that the extra overhead is
about 0.5ns when no hooks are added,
about 14ns when hooks are added,
and that each hook results in about 2ns of additional overhead.

Details:
- No Hook Added: The additional overhead is one atomic load and one branch jump,
  which should be very lightweight in most cases.
- Hooks Added: There is a read/write lock (just some atomic operations in most cases),
  some additional function calls via pointers,
  and some copy operations to pack parameters into a tuple.


