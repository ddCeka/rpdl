use {
    proc_macro2::TokenStream,
    quote::quote,
    syn::{Error, ItemFn, Result, ReturnType, Token, parse::Parse},
};

pub struct MainConfig {
    is_async: bool,
}

impl Parse for MainConfig {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Ok(MainConfig { is_async: false });
        }

        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Ident) {
            let ident: syn::Ident = input.parse()?;
            if ident == "async" {
                return Ok(MainConfig { is_async: true });
            }
            return Err(Error::new_spanned(
                ident,
                "expected `async` or no parameters",
            ));
        }

        Err(lookahead.error())
    }
}

pub(crate) fn expand_main(config: MainConfig, mut input: ItemFn) -> Result<TokenStream> {
    let original_name = &input.sig.ident;

    if !input.sig.inputs.is_empty() {
        return Err(Error::new_spanned(
            &input.sig.inputs,
            "main function with #[macror::main] cannot have parameters",
        ));
    }

    let fn_vis = &input.vis;
    let fn_attrs = &input.attrs;
    let fn_block = &input.block;
    let stmts = &fn_block.stmts;

    let return_type = match &input.sig.output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    let renamed_fn = quote::format_ident!("__macror_inner_{}", original_name);
    let needs_ok_wrap = matches!(&input.sig.output, ReturnType::Default);

    let eyre_setup = quote! {
        color_eyre::install()?;
    };

    let inner_body = if needs_ok_wrap {
        quote! {
            #eyre_setup
            #(#stmts)*
            ::std::result::Result::Ok(())
        }
    } else {
        quote! {
            #eyre_setup
            #(#stmts)*
        }
    };

    let new_return_type = if needs_ok_wrap {
        quote! { -> color_eyre::Result<()> }
    } else {
        match &input.sig.output {
            ReturnType::Default => quote! { -> color_eyre::Result<()> },
            ReturnType::Type(arrow, _) => quote! { #arrow color_eyre::Result<#return_type> },
        }
    };

    if config.is_async {
        if input.sig.asyncness.is_none() {
            input.sig.asyncness = Some(Token![async](proc_macro2::Span::call_site()));
        }

        Ok(quote! {
            #(#fn_attrs)*
            async fn #renamed_fn() #new_return_type {
                #inner_body
            }

            #fn_vis fn main() {
                ::tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build Tokio runtime")
                    .block_on(async {
                        if let Err(e) = #renamed_fn().await {
                            eprintln!("Error: {:?}", e);
                            ::std::process::exit(1);
                        }
                    })
            }
        })
    } else {
        if input.sig.asyncness.is_some() {
            return Err(Error::new_spanned(
                input.sig.asyncness,
                "function is async but `async` parameter not specified in macro",
            ));
        }

        Ok(quote! {
            #(#fn_attrs)*
            fn #renamed_fn() #new_return_type {
                #inner_body
            }

            #fn_vis fn main() {
                if let Err(e) = #renamed_fn() {
                    eprintln!("Error: {:?}", e);
                    ::std::process::exit(1);
                }
            }
        })
    }
}