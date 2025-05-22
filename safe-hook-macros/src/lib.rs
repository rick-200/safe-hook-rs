//! This create provides a convenient macro [macro@hookable] to mark functions, allowing them to be hooked.
//! 
//! See `safe-hook` crate for more details.



use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::parse::Parse;
use syn::{ItemFn, LitStr, parse_macro_input};
struct HookableProcArgs {
    name: LitStr,
    // args: Punctuated<MetaNameValue, Token![,]>,
}

impl Parse for HookableProcArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse::<LitStr>()?;
        // let args = Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input)?;
        Ok(HookableProcArgs {
            name,
            // args
        })
    }
}

fn gen_args_name_list(f: &ItemFn) -> proc_macro2::TokenStream {
    // fn xxx(a:ta,b:tb,c:tc) -> td;  ==> a,b,c
    let mut args = Vec::new();
    for arg in f.sig.inputs.iter() {
        if let syn::FnArg::Typed(pat_type) = arg {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                args.push(pat_ident.ident.clone());
            } else {
                panic!("Argument pattern is not supported");
            }
        }
    }
    quote! {
        #(#args),*
    }
}

fn get_hookable_lifetime(f: &ItemFn) -> Option<proc_macro2::TokenStream> {
    if f.sig.generics.where_clause.is_some() {
        panic!("Where clause is not supported");
    }
    match f.sig.generics.params.iter().count() {
        0 => None,
        1 => {
            let g = f.sig.generics.params.iter().next().unwrap();
            if let syn::GenericParam::Lifetime(lifetime) = g {
                Some(quote! { #lifetime })
            } else {
                panic!(
                    "Hookable cannot be used with generic '{}'",
                    g.to_token_stream()
                );
            }
        }
        _ => panic!(
            "Hookable cannot be used with more than one generics <{}>",
            f.sig.generics.params.to_token_stream()
        ),
    }
}
/// This macro is used to mark a function as hookable, without changing the signature.
/// It generates some extra codes to support hooks, and registers the function to the inventory.
/// 
/// Not Supported:
/// - functions with generic types
/// - functions with `self` receiver
/// - functions returns references
/// 
/// # Examples:
/// ```
/// #[hookable("add")]
/// fn add(left: i64, right: i64) -> i64 {
///    left + right
/// }
/// ```
#[proc_macro_attribute]
pub fn hookable(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as HookableProcArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let input_fn_ident = input_fn.sig.ident.clone();

    let _ = get_hookable_lifetime(&input_fn);
    let generics = input_fn.sig.generics.clone();

    let input_type = input_fn
        .sig
        .inputs
        .iter()
        .map(|arg| match arg {
            syn::FnArg::Typed(pat_type) => {
                if !matches!(&*pat_type.pat, syn::Pat::Ident(_)) {
                    panic!("Argument pattern is not supported");
                }
                pat_type.ty.clone()
            }
            syn::FnArg::Receiver(_) => panic!("Method receiver (self) is not supported"),
        })
        .collect::<Vec<_>>();
    let input_type_with_static_lifetime = input_type
        .iter()
        .map(|ty| {
            if let syn::Type::Reference(ref_ty) = &**ty {
                let mut ref_ty = ref_ty.clone();
                ref_ty.lifetime = Some(syn::Lifetime::new("'static", proc_macro2::Span::call_site()));
                quote! { #ref_ty }
            } else {
                quote! { #ty }
            }
        })
        .collect::<Vec<_>>();

    let ret_type = match &input_fn.sig.output {
        syn::ReturnType::Default => quote! { () },
        syn::ReturnType::Type(_, ty) => quote! { #ty },
    };

    let func_type = quote! {
        fn(#(#input_type),*) -> #ret_type
    };

    let hookable_name = args.name;

    let args_name_list = gen_args_name_list(&input_fn);

    let mut inner_fn = input_fn.clone();
    inner_fn.sig.ident = format_ident!("__hookable_inner");
    let fn_sig = &input_fn.sig;

    let unpack_list: proc_macro2::TokenStream = (0..input_fn.sig.inputs.len())
        .map(|i| {
            let idx = syn::Index::from(i);
            quote! { args.#idx, }
        })
        .collect();

    // 原样返回函数代码
    let generated = quote! {
        #fn_sig {
            #inner_fn

            use safe_hook::HookableFuncMetadata;
            use core::sync::atomic::AtomicBool;
            use std::sync::LazyLock;
            use std::sync::atomic::Ordering;

            type SelfFunc #generics = #func_type;

            static FLAG: AtomicBool = AtomicBool::new(false);
            static META: LazyLock<HookableFuncMetadata> = LazyLock::new(|| {
                let metadata = unsafe {
                    HookableFuncMetadata::new(
                        #hookable_name.to_string(),
                        #input_fn_ident as *const (),
                        (
                            std::any::TypeId::of::<#ret_type>(),
                            std::any::TypeId::of::<(#(#input_type_with_static_lifetime),*)>(),
                        ),
                        &FLAG,
                    )
                };
                metadata
            });
            safe_hook::inventory::submit! {
                safe_hook::HookableFuncRegistry::new(&META)
            }
            if !FLAG.load(Ordering::Acquire) {
                return __hookable_inner(#args_name_list);
            }
            safe_hook::call_with_hook::<#ret_type, (#(#input_type),*)>(|args| __hookable_inner(#unpack_list), &META, (#args_name_list))
        }
    };
    generated.into()
}
