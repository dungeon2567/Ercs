use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemFn, FnArg, PatType, Type, TypePath, TypeReference, PathArguments, GenericArgument, Item, Meta, Expr, ExprLit, Lit};

fn pascalize(s: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in s.chars() {
        if ch == '_' || ch == '-' {
            capitalize = true;
            continue;
        }
        if capitalize {
            out.extend(ch.to_uppercase());
            capitalize = false;
        } else {
            out.push(ch);
        }
    }
    out
}

#[proc_macro_attribute]
pub fn system(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let fn_ident = func.sig.ident.clone();
    let mut override_name: Option<String> = None;
    if !attr.is_empty() {
        let meta = parse_macro_input!(attr as Meta);
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("name") {
                if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = nv.value {
                    override_name = Some(s.value());
                }
            }
        }
    }

    let mut arg_types: Vec<syn::Type> = Vec::new();
    for arg in func.sig.inputs.iter() {
        if let FnArg::Typed(PatType { ty, .. }) = arg {
            if let Type::Reference(TypeReference { elem, .. }) = &**ty {
                if let Type::Path(TypePath { path, .. }) = &**elem {
                    let last = path.segments.last().expect("empty path");
                    if last.ident == "View" {
                        if let PathArguments::AngleBracketed(ab) = &last.arguments {
                            if let Some(GenericArgument::Type(inner_ty)) = ab.args.first() {
                                arg_types.push(inner_ty.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(arg_types.len() == 2, "#[system] expects function signature like fn f(a: &View<A>, b: &View<B>)");
    let ty_a = arg_types.remove(0);
    let ty_b = arg_types.remove(0);

    let struct_ident = match override_name {
        Some(n) => format_ident!("{}", n),
        None => {
            let base = pascalize(&fn_ident.to_string());
            format_ident!("{}System", base)
        }
    };

    let expanded = quote! {
        #func

        pub struct #struct_ident {
            a: std::rc::Rc<std::cell::RefCell<crate::storage::storage::SparseStorage<#ty_a, std::alloc::Global>>>,
            b: std::rc::Rc<std::cell::RefCell<crate::storage::storage::SparseStorage<#ty_b, std::alloc::Global>>>,
        }

        impl #struct_ident {
            pub fn new(world: &mut crate::world::World) -> Self {
                let a = world.get::<#ty_a>();
                let b = world.get::<#ty_b>();
                Self { a, b }
            }
        }

        impl crate::system::system::System for #struct_ident {}

        impl crate::scheduler::PipelineStage for #struct_ident {
            fn run(&self) {
                use crate::view::iter::{IterViews, intersect};
                let a_cell = self.a.borrow();
                let b_cell = self.b.borrow();
                let a_store = &a_cell.root;
                let b_store = &b_cell.root;

                for (a_view, b_view) in intersect(a_store.views(), b_store.views()) {
                    #fn_ident(&a_view, &b_view);
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn derive_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as Item);
    let ident = match &ast {
        Item::Struct(s) => s.ident.clone(),
        Item::Enum(e) => e.ident.clone(),
        _ => panic!("derive_component supports struct or enum"),
    };

    let expanded = quote! {
        #ast
        impl crate::component::Component for #ident {}
    };
    TokenStream::from(expanded)
}
