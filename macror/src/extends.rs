use {
    proc_macro2::TokenStream,
    quote::quote,
    syn::{Ident, ItemFn, Result, parse::Parse},
};

pub(crate) struct ExtendsConfig {
    target: Ident,
}

impl Parse for ExtendsConfig {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let target: Ident = input.parse()?;
        Ok(ExtendsConfig { target })
    }
}

pub(crate) fn expand_extends(config: ExtendsConfig, input: ItemFn) -> Result<TokenStream> {
    let target = &config.target;
    let fn_vis = &input.vis;
    let fn_sig = &input.sig;
    let fn_name = &fn_sig.ident;
    let fn_block = &input.block;
    let fn_attrs = &input.attrs;

    let inputs = &fn_sig.inputs;
    let output = &fn_sig.output;
    let asyncness = &fn_sig.asyncness;
    let unsafety = &fn_sig.unsafety;
    let generics = &fn_sig.generics;
    let where_clause = &fn_sig.generics.where_clause;

    Ok(quote! {
        impl #target {
            #(#fn_attrs)*
            #fn_vis #unsafety #asyncness fn #fn_name #generics(#inputs) #output #where_clause {
                #fn_block
            }
        }
    })
}
