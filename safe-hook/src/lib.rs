pub use inventory;
pub use safe_hook_macros::hookable;
use std::any::TypeId;
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock, RwLock};

pub trait Hook: Send + Sync + 'static {
    type Args<'b>;
    type Result;
    fn call<'a>(
        &'a self,
        args: Self::Args<'a>,
        next: &dyn for<'c> Fn(Self::Args<'c>) -> Self::Result,
    ) -> Self::Result;
}

///
/// # Safety
/// This trait should never be implemented by hand.
pub unsafe trait HookDyn: Send + Sync {
    fn get_call_fn(&self) -> *const ();
    fn type_info(&self) -> (TypeId, TypeId);
}

// 为了防止&T和*const ()的调用约定不同，增加一个包装层
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

pub struct HookableFuncRegistry {
    metadata: &'static LazyLock<HookableFuncMetadata>,
}
impl HookableFuncRegistry {
    pub const fn new(metadata: &'static LazyLock<HookableFuncMetadata>) -> Self {
        Self { metadata }
    }
}

inventory::collect!(HookableFuncRegistry);

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

pub struct HookableFuncMetadata {
    name: String,
    func: HookableFuncPtr,
    type_info: (TypeId, TypeId),
    fast_path_flag: &'static AtomicBool,
    hooks: RwLock<Vec<(Arc<dyn HookDyn>, i32)>>,
}
impl HookableFuncMetadata {
    ///
    /// # Safety
    /// This function is unsafe because it takes a raw pointer to a function without type checking.
    /// It is used inside the macro `hookable!` to create a new `HookableFuncMetadata` instance.
    /// DO NOT USE THIS FUNCTION UNLESS YOU KNOW WHAT YOU ARE DOING
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn func_ptr(&self) -> *const () {
        self.func.0
    }

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

    pub fn add_hook(&self, hook: Arc<dyn HookDyn>) -> Result<(), String> {
        self.add_hook_with_priority(hook, 0)
    }

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

    pub fn clear_hooks(&self) {
        let mut hooks = self.hooks.write().unwrap();
        hooks.clear();
        self.fast_path_flag
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

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
