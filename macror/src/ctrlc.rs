use {
    proc_macro2::TokenStream,
    quote::quote,
    syn::{Error, ItemFn, Result},
};

pub(crate) fn expand_ctrlc(input: ItemFn) -> Result<TokenStream> {
    let fn_vis = &input.vis;
    let fn_sig = &input.sig;
    let fn_name = &fn_sig.ident;
    let fn_block = &input.block;
    let fn_attrs = &input.attrs;

    if !fn_sig.inputs.is_empty() {
        return Err(Error::new_spanned(
            &fn_sig.inputs,
            "ctrlc handler function must not have any parameters",
        ));
    }

    match &fn_sig.output {
        syn::ReturnType::Default => {}
        syn::ReturnType::Type(_, ty) => {
            if let syn::Type::Tuple(tuple) = &**ty {
                if !tuple.elems.is_empty() {
                    return Err(Error::new_spanned(
                        ty,
                        "ctrlc handler function must return () or have no return type",
                    ));
                }
            } else {
                return Err(Error::new_spanned(
                    ty,
                    "ctrlc handler function must return () or have no return type",
                ));
            }
        }
    }

    let wrapper_name = quote::format_ident!("__ctrlc_wrapper_{}", fn_name);

    Ok(quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            fn #wrapper_name() {
                #fn_block
            }

            ::ctrlc::set_handler(move || {
                #wrapper_name();
            })
            .expect("Error setting Ctrl-C handler");
        }
    })
}
