pub use inventory;
pub use safe_hook_macros::hookable;
use std::any::TypeId;
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock, RwLock};

pub trait FnSig {
    type Args;
    type Result;

    fn call(self, args: Self::Args) -> Self::Result;
}

macro_rules! impl_fn_sig {
    ($($arg_t:ident $arg:ident),+) => {
        impl <R, $( $arg_t ),*> FnSig for fn($($arg_t),*) -> R {
            // rustc false positive unused_parens
            #[allow(unused_parens)]
            type Args = ( $($arg_t),* );
            type Result = R;
            fn call(
                self,
                args: Self::Args
            ) -> Self::Result {
                // rustc false positive unused_parens
                #[allow(unused_parens)]
                let ( $($arg),* ) = args;
                self( $($arg),* )
            }
        }
    };
}
impl<R> FnSig for fn() -> R {
    type Args = ();
    type Result = R;
    fn call(self, _args: Self::Args) -> Self::Result {
        self()
    }
}


impl_fn_sig!(T1 x1);
impl_fn_sig!(T1 x1, T2 x2);
impl_fn_sig!(T1 x1, T2 x2, T3 x3);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7, T8 x8);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7, T8 x8, T9 x9);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7, T8 x8, T9 x9, T10 x10);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7, T8 x8, T9 x9, T10 x10, T11 x11);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7, T8 x8, T9 x9, T10 x10, T11 x11, T12 x12);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7, T8 x8, T9 x9, T10 x10, T11 x11, T12 x12, T13 x13);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7, T8 x8, T9 x9, T10 x10, T11 x11, T12 x12, T13 x13, T14 x14);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7, T8 x8, T9 x9, T10 x10, T11 x11, T12 x12, T13 x13, T14 x14, T15 x15);
impl_fn_sig!(T1 x1, T2 x2, T3 x3, T4 x4, T5 x5, T6 x6, T7 x7, T8 x8, T9 x9, T10 x10, T11 x11, T12 x12, T13 x13, T14 x14, T15 x15, T16 x16);

pub trait Hook: Send + Sync {
    type Func: FnSig;
    fn call(
        &self,
        args: <Self::Func as FnSig>::Args,
        next: &dyn Fn(<Self::Func as FnSig>::Args) -> <Self::Func as FnSig>::Result,
    ) -> <Self::Func as FnSig>::Result;
}

///
/// # Safety
/// This trait should never be implemented by hand.
pub unsafe trait HookDyn: Send + Sync {
    fn get_call_fn(&self) -> *const ();
    fn type_info(&self) -> (TypeId, TypeId);
}

// 为了防止&T和*const ()的调用约定不同，增加一个包装层
unsafe fn hook_call_wrapper<T: Hook + 'static>(
    self_ptr: *const (),
    args: <<T as Hook>::Func as FnSig>::Args,
    next: &dyn Fn(<<T as Hook>::Func as FnSig>::Args) -> <<T as Hook>::Func as FnSig>::Result,
) -> <<T as Hook>::Func as FnSig>::Result {
    let self_ref = unsafe { &*(self_ptr as *const T) };
    self_ref.call(args, next)
}

unsafe impl<T: Hook + 'static> HookDyn for T {
    fn get_call_fn(&self) -> *const () {
        hook_call_wrapper::<T> as *const ()
    }
    fn type_info(&self) -> (TypeId, TypeId) {
        let res = TypeId::of::<<<T as Hook>::Func as FnSig>::Result>();
        let args = TypeId::of::<<<T as Hook>::Func as FnSig>::Args>();
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
            .store(true, std::sync::atomic::Ordering::Relaxed);
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

pub fn call_with_hook<F: FnSig + Copy>(
    func: F,
    meta: &'static HookableFuncMetadata,
    args: F::Args,
) -> F::Result {
    let hooks = meta.hooks.read().unwrap();
    let pos = Cell::new(0);
    #[allow(clippy::type_complexity)]
    let next_fn_ref: Cell<Option<&dyn Fn(F::Args) -> F::Result>> = Cell::new(None);
    type HookFn<T> = fn(
        *const (),
        args: <T as FnSig>::Args,
        next: &dyn Fn(<T as FnSig>::Args) -> <T as FnSig>::Result,
    ) -> <T as FnSig>::Result;
    let next_fn = |args: F::Args| {
        if pos.get() < hooks.len() {
            let hook = hooks[pos.get()].0.as_ref();
            let f: HookFn<F> = unsafe { std::mem::transmute(hook.get_call_fn()) };
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
            func.call(args)
        }
    };
    next_fn_ref.set(Some(&next_fn));
    next_fn(args)
}
