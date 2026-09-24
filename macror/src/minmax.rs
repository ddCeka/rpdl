use {
    proc_macro2::TokenStream,
    quote::quote,
    syn::{
        Error, Expr, FnArg, ItemFn, Lit, Meta, Pat, Result, Token, Type,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
    },
};

struct MinMaxConfig {
    min: Option<i128>,
    max: Option<i128>,
}

impl Parse for MinMaxConfig {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut config = MinMaxConfig {
            min: None,
            max: None,
        };

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("min") => {
                    if config.min.is_some() {
                        return Err(Error::new_spanned(nv, "duplicate `min` attribute"));
                    }

                    config.min = Some(parse_int_literal(&nv.value)?);
                }

                Meta::NameValue(nv) if nv.path.is_ident("max") => {
                    if config.max.is_some() {
                        return Err(Error::new_spanned(nv, "duplicate `max` attribute"));
                    }
                    config.max = Some(parse_int_literal(&nv.value)?);
                }

                _ => {
                    return Err(Error::new_spanned(
                        meta,
                        "expected `min = <value>` or `max = <value>`",
                    ));
                }
            }
        }

        if config.min.is_none() && config.max.is_none() {
            return Err(Error::new(
                input.span(),
                "at least one of `min` or `max` must be specified",
            ));
        }

        if let (Some(min), Some(max)) = (config.min, config.max)
            && min > max
        {
            return Err(Error::new(
                input.span(),
                format!("min ({}) cannot be greater than max ({})", min, max),
            ));
        }

        Ok(config)
    }
}

fn parse_int_literal(expr: &Expr) -> Result<i128> {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Int(lit_int) => lit_int
                .base10_parse::<i128>()
                .map_err(|_| Error::new_spanned(lit_int, "invalid integer literal")),
            _ => Err(Error::new_spanned(expr_lit, "expected integer literal")),
        },
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            if let Expr::Lit(expr_lit) = &*unary.expr
                && let Lit::Int(lit_int) = &expr_lit.lit
            {
                let val = lit_int
                    .base10_parse::<i128>()
                    .map_err(|_| Error::new_spanned(lit_int, "invalid integer literal"))?;
                return Ok(-val);
            }
            Err(Error::new_spanned(unary, "expected integer literal"))
        }
        _ => Err(Error::new_spanned(expr, "expected integer literal")),
    }
}

pub(crate) fn expand_minmax(input: ItemFn) -> Result<TokenStream> {
    let mut fn_item = input;
    let mut checks = Vec::new();

    for (idx, arg) in fn_item.sig.inputs.iter_mut().enumerate() {
        if let FnArg::Typed(pat_type) = arg {
            let mut minmax_attr = None;

            pat_type.attrs.retain(|attr| {
                if attr.path().is_ident("minmax") {
                    minmax_attr = Some(attr.clone());
                    false
                } else {
                    true
                }
            });

            if let Some(attr) = minmax_attr {
                let config: MinMaxConfig = attr.parse_args()?;

                let param_name = match &*pat_type.pat {
                    Pat::Ident(ident) => &ident.ident,
                    _ => {
                        return Err(Error::new_spanned(
                            &pat_type.pat,
                            "minmax requires a simple identifier pattern",
                        ));
                    }
                };

                let param_type = &pat_type.ty;

                if !is_integer_type(param_type) {
                    return Err(Error::new_spanned(
                        param_type,
                        "minmax can only be applied to integer types (i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize)",
                    ));
                }

                let check_name =
                    quote::format_ident!("__minmax_check_{}_{}", fn_item.sig.ident, idx);

                let mut conditions = Vec::new();

                if let Some(min) = config.min {
                    let min_lit =
                        syn::LitInt::new(&min.to_string(), proc_macro2::Span::call_site());
                    conditions.push(quote! {
                        (#param_name as i128) < #min_lit
                    });
                }

                if let Some(max) = config.max {
                    let max_lit =
                        syn::LitInt::new(&max.to_string(), proc_macro2::Span::call_site());
                    conditions.push(quote! {
                        (#param_name as i128) > #max_lit
                    });
                }

                let condition = if conditions.len() == 1 {
                    conditions[0].clone()
                } else {
                    quote! { #(#conditions)||* }
                };

                let error_msg = match (config.min, config.max) {
                    (Some(min), Some(max)) => {
                        format!(
                            "parameter `{}` must be between {} and {}",
                            param_name, min, max
                        )
                    }
                    (Some(min), None) => {
                        format!("parameter `{}` must be at least {}", param_name, min)
                    }
                    (None, Some(max)) => {
                        format!("parameter `{}` must be at most {}", param_name, max)
                    }
                    (None, None) => unreachable!(),
                };

                checks.push(quote! {
                    const fn #check_name(#param_name: #param_type) {
                        if #condition {
                            panic!(#error_msg);
                        }
                    }
                });
            }
        }
    }

    let vis = &fn_item.vis;
    let sig = &fn_item.sig;
    let block = &fn_item.block;
    let attrs = &fn_item.attrs;

    Ok(quote! {
        #(#checks)*

        #(#attrs)*
        #vis #sig #block
    })
}

fn is_integer_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(ident) = type_path.path.get_ident()
    {
        let ident_str = ident.to_string();
        return matches!(
            ident_str.as_str(),
            "i8" | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
        );
    }
    false
}
