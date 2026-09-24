#![warn(clippy::get_first, clippy::index_refutable_slice)]
use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{
        BinOp, Expr, ExprBinary, ExprLit, Ident, ItemFn, Lit,
        parse::{Parse, ParseStream},
        parse_macro_input,
        visit_mut::VisitMut,
    },
};

pub(super) fn expand_math(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);

    let mode = if attr.is_empty() {
        OverflowMode::Checked
    } else {
        match syn::parse::<OverflowMode>(attr) {
            Ok(mode) => mode,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    let mut transformer = Mathinator2000 { mode };
    transformer.visit_item_fn_mut(&mut input);

    TokenStream::from(quote! { #input })
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum OverflowMode {
    Checked,
    Saturating,
    Wrapping,
}

impl Parse for OverflowMode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "checked" => Ok(OverflowMode::Checked),
            "saturating" => Ok(OverflowMode::Saturating),
            "wrapping" => Ok(OverflowMode::Wrapping),
            _ => Err(syn::Error::new_spanned(
                ident,
                "expected `checked`, `saturating`, or `wrapping`",
            )),
        }
    }
}

struct Mathinator2000 {
    mode: OverflowMode,
}

impl VisitMut for Mathinator2000 {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        syn::visit_mut::visit_expr_mut(self, expr);

        if let Expr::Binary(binary_expr) = expr
            && let Some(new_expr) = self.transform_binary_expr(binary_expr)
        {
            *expr = new_expr;
        }
    }
}

impl Mathinator2000 {
    pub(crate) fn is_float_literal(expr: &Expr) -> bool {
        !matches!(
            expr,
            Expr::Lit(ExprLit {
                lit: Lit::Float(_),
                ..
            })
        )
    }

    fn transform_binary_expr(&self, binary: &ExprBinary) -> Option<Expr> {
        let left = &binary.left;
        let right = &binary.right;

        if Self::is_float_literal(left) || Self::is_float_literal(right) {
            return None;
        }

        let is_compound_assignment = matches!(
            binary.op,
            BinOp::AddAssign(_)
                | BinOp::SubAssign(_)
                | BinOp::MulAssign(_)
                | BinOp::DivAssign(_)
                | BinOp::RemAssign(_)
        );

        let (base_method, op_symbol) = match binary.op {
            BinOp::Add(_) | BinOp::AddAssign(_) => ("add", "+"),
            BinOp::Sub(_) | BinOp::SubAssign(_) => ("sub", "-"),
            BinOp::Mul(_) | BinOp::MulAssign(_) => ("mul", "*"),
            BinOp::Div(_) | BinOp::DivAssign(_) => ("div", "/"),
            BinOp::Rem(_) | BinOp::RemAssign(_) => ("rem", "%"),
            _ => return None,
        };

        if is_compound_assignment {
            match self.mode {
                OverflowMode::Checked => {
                    let method = format!("checked_{}", base_method);
                    let method_ident = Ident::new(&method, proc_macro2::Span::call_site());

                    Some(syn::parse_quote! {
                        #left = #left.#method_ident(#right)
                            .expect(&::std::format!(
                                "Arithmetic overflow: {} {}= {} exceeds type bounds at {}:{}",
                                #left,
                                #op_symbol,
                                #right,
                                ::std::file!(),
                                ::std::line!()
                            ))
                    })
                }
                OverflowMode::Saturating => {
                    if base_method == "rem" {
                        return None;
                    }

                    let method = format!("saturating_{}", base_method);
                    let method_ident = Ident::new(&method, proc_macro2::Span::call_site());

                    Some(syn::parse_quote! {
                        #left = (#left).#method_ident(#right)
                    })
                }
                OverflowMode::Wrapping => {
                    let method = format!("wrapping_{}", base_method);
                    let method_ident = Ident::new(&method, proc_macro2::Span::call_site());

                    Some(syn::parse_quote! {
                        #left = (#left).#method_ident(#right)
                    })
                }
            }
        } else {
            match self.mode {
                OverflowMode::Checked => {
                    let method = format!("checked_{}", base_method);
                    let method_ident = Ident::new(&method, proc_macro2::Span::call_site());

                    Some(syn::parse_quote! {
                        {
                            let __left_val = #left;
                            let __right_val = #right;
                            __left_val.#method_ident(__right_val)
                                .expect(&::std::format!(
                                    "Arithmetic overflow: {} {} {} exceeds type bounds at {}:{}",
                                    __left_val,
                                    #op_symbol,
                                    __right_val,
                                    ::std::file!(),
                                    ::std::line!()
                                ))
                        }
                    })
                }
                OverflowMode::Saturating => {
                    if base_method == "rem" {
                        return None;
                    }

                    let method = format!("saturating_{}", base_method);
                    let method_ident = Ident::new(&method, proc_macro2::Span::call_site());

                    Some(syn::parse_quote! {
                        (#left).#method_ident(#right)
                    })
                }
                OverflowMode::Wrapping => {
                    let method = format!("wrapping_{}", base_method);
                    let method_ident = Ident::new(&method, proc_macro2::Span::call_site());

                    Some(syn::parse_quote! {
                        (#left).#method_ident(#right)
                    })
                }
            }
        }
    }
}