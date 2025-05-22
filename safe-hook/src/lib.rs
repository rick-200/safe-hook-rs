//! Safe-Hook is an inline hook library for Rust.
//! It provides a simple and safe way to create hooks in your Rust applications,
//! allowing you to modify the behavior of functions at runtime.
//! 
//! The design principle of Safe-Hook is safety and simplicity.
//! 
//! ## Features
//! - **Inline Hooking**: Safe-Hook allows you to hook into functions at runtime,
//!   enabling you to modify their behavior.
//! - **Safe and Simple**: The library is designed to be safe and easy to use,
//!   it checks types of parameters and return values at runtime to ensure safety.
//! - **Full Dynamic**: Safe-Hook is fully dynamic,
//!   allowing you to add and remove hooks at runtime without any restrictions.
//! - **Cross-Platform**: Safe-Hook is designed to work on multiple platforms,
//!   it theoretically supports all platforms that Rust supports.
//! 
//! ## Usage
//! For more examples, please refer to `examples` and `tests` directory.
//! ```rust
//! use std::sync::Arc;
//! use safe_hook::{lookup_hookable, Hook};
//! use safe_hook_macros::hookable;
//! 
//! #[hookable("add")]
//! fn add(left: i64, right: i64) -> i64 {
//!     left + right
//! }
//! 
//! #[derive(Debug)]
//! struct HookAdd {
//!     x: i64,
//! }
//! 
//! impl Hook for HookAdd {
//!     type Args<'a> = (i64, i64);
//!     type Result = i64;
//!     fn call(&self, args: (i64, i64), next: &dyn Fn((i64, i64)) -> i64) -> i64 {
//!         next(args) + self.x
//!     }
//! }
//! 
//! fn main() {
//!     let hook = Arc::new(HookAdd {
//!         x: 1,
//!     });
//!     assert_eq!(add(1, 2), 3);
//!     lookup_hookable("add").unwrap().add_hook(hook).unwrap();
//!     assert_eq!(add(1, 2), 4);
//! }
//! ```
//! 
//! ## Limitations
//! - **Intrusive**: Needs to annotate target functions manually.
//!   Which means it's not suitable for hook third-party libraries.
//! 
//! ## Performance
//! Extra overhead:
//! - No Hook Added: One atomic load and one branch jump,
//!   which should be very lightweight in most cases.
//! - Hooks Added: There is a read/write lock (just some atomic operations in most cases),
//!   some additional function calls via pointers,
//!   and some copy operations to pack parameters into a tuple.
//! 
//! A sloppy benchmark (uses 12700H) shows that the extra overhead is
//! about 0.5ns when no hooks are added 
//! (as a comparison, an `add(a,b)` function takes about 0.5ns),
//! about 14ns when hooks are added,
//! and that each additional hook results in about 2ns of overhead.

use std::any::TypeId;
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock, RwLock};

#[doc(hidden)]
pub use inventory;

pub use safe_hook_macros::hookable;
/// A Trait for hooks.
/// Implements this trait to create a hook.
pub trait Hook: Send + Sync + 'static {
    /// The arguments type of the hook. Must be a tuple.
    /// Must be the same as the arguments of the target hookable function you want to hook.
    type Args<'b>;

    /// The result type of the hook.
    /// Must be the same as the result of the target hookable function you want to hook.
    type Result;

    /// The hook function.
    /// This will be called when the target function is called.
    /// # Parameters:
    /// - `args`: The arguments of the target function.
    /// - `next`: The next function to call. This is the next hook or original target function.
    fn call<'a>(
        &'a self,
        args: Self::Args<'a>,
        next: &dyn for<'c> Fn(Self::Args<'c>) -> Self::Result,
    ) -> Self::Result;
}

/// A trait for dynamic dispatch of hooks.
/// # Safety
/// This trait should never be implemented by hand.
#[doc(hidden)]
pub unsafe trait HookDyn: Send + Sync {
    fn get_call_fn(&self) -> *const ();
    fn type_info(&self) -> (TypeId, TypeId);
}

/// A wrapper layer to avoid the calling convention difference between &T and *const ().
unsafe fn hook_call_wrapper<'a, T: Hook + 'static>(
    self_ptr: *const (),
    args: <T as Hook>::Args<'a>,
    next: &dyn for<'b> Fn(<T as Hook>::Args<'b>) -> <T as Hook>::Result,
) -> <T as Hook>::Result {
    let self_ref = unsafe { &*(self_ptr as *const T) };
    self_ref.call(args, next)
}

unsafe impl<T: Hook + 'static> HookDyn for T {
    fn get_call_fn(&self) -> *const () {
        hook_call_wrapper::<T> as *const ()
    }
    fn type_info(&self) -> (TypeId, TypeId) {
        let res = TypeId::of::<<T as Hook>::Result>();
        let args = TypeId::of::<<T as Hook>::Args<'static>>();
        (res, args)
    }
}

/// A registry entry for hookable functions.
#[doc(hidden)]
pub struct HookableFuncRegistry {
    metadata: &'static LazyLock<HookableFuncMetadata>,
}
impl HookableFuncRegistry {
    pub const fn new(metadata: &'static LazyLock<HookableFuncMetadata>) -> Self {
        Self { metadata }
    }
}

inventory::collect!(HookableFuncRegistry);

/// Lookup a hookable function by name.
pub fn lookup_hookable(name: &str) -> Option<&'static HookableFuncMetadata> {
    // struct MyHashBuilder;
    // impl BuildHasher for MyHashBuilder {
    //     type Hasher = DefaultHasher;
    //     fn build_hasher(&self) -> Self::Hasher {
    //         DefaultHasher::new()
    //     }
    // }
    // static CACHE: Mutex<HashMap<String, &'static LazyLock<HookableFuncMetadata>, MyHashBuilder>> = Mutex::new(HashMap::with_hasher(MyHashBuilder{}));

    for item in inventory::iter::<HookableFuncRegistry> {
        if item.metadata.name == name {
            return Some(item.metadata);
        }
    }
    None
}

struct HookableFuncPtr(*const ());
unsafe impl Send for HookableFuncPtr {}
unsafe impl Sync for HookableFuncPtr {}

/// Metadata for a hookable function.
#[doc(hidden)]
pub struct HookableFuncMetadata {
    name: String,
    func: HookableFuncPtr,
    type_info: (TypeId, TypeId),
    fast_path_flag: &'static AtomicBool,
    hooks: RwLock<Vec<(Arc<dyn HookDyn>, i32)>>,
}
impl HookableFuncMetadata {
    /// Create a new [`HookableFuncMetadata`].
    /// # Safety
    /// This function is unsafe because it takes a raw pointer to a function without type checking.
    /// It is used inside the macro [`hookable`] to create a new [`HookableFuncMetadata`] instance.
    /// DO NOT USE THIS FUNCTION UNLESS YOU KNOW WHAT YOU ARE DOING
    #[doc(hidden)]
    pub unsafe fn new(
        name: String,
        func: *const (),
        type_info: (TypeId, TypeId),
        fast_path_flag: &'static AtomicBool,
    ) -> Self {
        Self {
            name,
            func: HookableFuncPtr(func),
            type_info,
            fast_path_flag,
            hooks: RwLock::new(Vec::new()),
        }
    }

    /// Get the name of the hookable function.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the pointer to the hookable function.
    pub fn func_ptr(&self) -> *const () {
        self.func.0
    }

    /// Add a hook to the hookable function.
    /// The greatest priority will be called first.
    pub fn add_hook_with_priority(
        &self,
        hook: Arc<dyn HookDyn>,
        priority: i32,
    ) -> Result<(), String> {
        if hook.type_info() != self.type_info {
            return Err(format!(
                "Hook type mismatch: expected {:?}, got {:?}",
                self.type_info,
                hook.type_info()
            ));
        }
        let mut hooks = self.hooks.write().unwrap();
        let pos = hooks
            .iter()
            .position(|h| h.1 <= priority)
            .unwrap_or(hooks.len());
        hooks.insert(pos, (hook, priority));
        self.fast_path_flag
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }

    /// Add a hook to the hookable function with default (0) priority.
    pub fn add_hook(&self, hook: Arc<dyn HookDyn>) -> Result<(), String> {
        self.add_hook_with_priority(hook, 0)
    }

    /// Remove a hook from the hookable function.
    pub fn remove_hook(&self, hook: &dyn HookDyn) -> bool {
        let mut hooks = self.hooks.write().unwrap();
        if let Some(pos) = hooks
            .iter()
            .position(|h| std::ptr::addr_eq(h.0.as_ref(), hook))
        {
            hooks.remove(pos);
            if hooks.is_empty() {
                self.fast_path_flag
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            true
        } else {
            false
        }
    }

    /// Clear all hooks from the hookable function.
    pub fn clear_hooks(&self) {
        let mut hooks = self.hooks.write().unwrap();
        hooks.clear();
        self.fast_path_flag
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Call a hookable function with hooks.
#[doc(hidden)]
pub fn call_with_hook<R, A>(func: fn(A) -> R, meta: &'static HookableFuncMetadata, args: A) -> R {
    let hooks = meta.hooks.read().unwrap();
    let pos = Cell::new(0);
    #[allow(clippy::type_complexity)]
    let next_fn_ref: Cell<Option<&dyn Fn(A) -> R>> = Cell::new(None);
    type HookFn<A, R> = fn(*const (), args: A, next: &dyn Fn(A) -> R) -> R;
    let next_fn = |args: A| {
        if pos.get() < hooks.len() {
            let hook = hooks[pos.get()].0.as_ref();
            let f: HookFn<A, R> = unsafe { std::mem::transmute(hook.get_call_fn()) };
            pos.set(pos.get() + 1);
            let res = f(
                hook as *const dyn HookDyn as *const (),
                args,
                // SAFETY: next_fn_ref must be set before calling next_fn
                unsafe { next_fn_ref.get().unwrap_unchecked() },
            );
            pos.set(pos.get() - 1);
            res
        } else {
            func(args)
        }
    };
    next_fn_ref.set(Some(&next_fn));
    next_fn(args)
}
