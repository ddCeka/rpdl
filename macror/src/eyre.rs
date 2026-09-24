use {
    proc_macro2::TokenStream,
    quote::quote,
    syn::{ItemFn, Result, ReturnType},
};

pub(crate) fn expand_eyre(input: ItemFn) -> Result<TokenStream> {
    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;
    let stmts = &block.stmts;
    let fn_name = &sig.ident;

    if fn_name != "main" {
        return Err(syn::Error::new_spanned(
            fn_name,
            "the `eyre` attribute can only be applied to the main function",
        ));
    }

    let new_output = match &sig.output {
        ReturnType::Default => {
            quote! { -> color_eyre::Result<()> }
        }
        ReturnType::Type(arrow, ty) => {
            quote! { #arrow #ty }
        }
    };

    let asyncness = &sig.asyncness;
    let generics = &sig.generics;
    let where_clause = &sig.generics.where_clause;

    let new_body = match &sig.output {
        ReturnType::Default => {
            quote! {
                {
                    color_eyre::install()?;
                    #(#stmts)*
                    Ok(())
                }
            }
        }

        ReturnType::Type(_, _) => {
            quote! {
                {
                    color_eyre::install()?;
                    #(#stmts)*
                }
            }
        }
    };

    let result = quote! {
        #(#attrs)*
        #vis #asyncness fn #fn_name #generics() #new_output #where_clause #new_body
    };

    Ok(result)
}
