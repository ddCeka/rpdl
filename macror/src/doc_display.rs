use {
    proc_macro2::TokenStream,
    quote::{format_ident, quote},
    syn::{
        Data, DeriveInput, Fields, GenericParam, Generics, Ident, Meta, Result, Type, parse_quote,
        spanned::Spanned,
    },
};

fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(std::fmt::Display));
        }
    }

    generics
}

pub(crate) fn expand_doc_display(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let is_struct = matches!(input.data, Data::Struct(_));
    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let static_assertions = if is_struct {
        let type_params: Vec<_> = generics.type_params().map(|param| &param.ident).collect();

        quote! {
            #(
                const _: fn() || {
                    fn assert_display<T: std::fmt::Display>() {}
                    assert_display::<#type_params>();
                };
            )*
        }
    } else {
        quote! {}
    };

    let doc_str = extract_doc_comments(&input.attrs);

    if matches!(&input.data, Data::Struct(_)) && doc_str.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "DocDisplay requires doc comments on the main type for structs",
        ));
    }

    let display_impl = match &input.data {
        Data::Struct(data_struct) => generate_struct_display(&doc_str, &data_struct.fields),
        Data::Enum(data_enum) => generate_enum_display(name, &doc_str, data_enum),
        Data::Union(data_union) => {
            return Err(syn::Error::new_spanned(
                data_union.union_token,
                "DocDisplay cannot be derived for unions",
            ));
        }
    };

    let disp_trait_ident = format_ident!("{}_CheckDisplay", name.to_string());

    let expanded = quote! {
        pub trait #disp_trait_ident {
            fn disp(&self) -> String;
        }

        impl #impl_generics #disp_trait_ident for #name #ty_generics #where_clause {
            fn disp(&self) -> String {
                #static_assertions

                String::new()
            }
        }

        impl #impl_generics ::std::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                #display_impl
            }
        }
    };

    Ok(expanded)
}

fn generate_struct_display(doc_str: &str, fields: &Fields) -> proc_macro2::TokenStream {
    let field_refs = find_field_refs(doc_str);
    let field_names: Vec<String> = match fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                if let Type::Path(type_path) = &field.ty
                    && let Some(last_seg) = type_path.path.segments.last()
                    && (last_seg.ident == "Option" || last_seg.ident == "Result")
                {
                    return syn::Error::new(
                        field.ty.span(),
                        format!(
                            "DocDisplay doesn't support `{}`s yet",
                            field.ty.span().source_text().unwrap()
                        ),
                    )
                    .to_compile_error();
                }
            }

            fields_named
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
                .collect()
        }
        Fields::Unnamed(_) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "DocDisplay doesn't support tuple structs with field interpolation",
            )
            .to_compile_error();
        }
        Fields::Unit => Vec::new(),
    };

    for field_ref in &field_refs {
        if !field_names.contains(field_ref) {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Field `{}` referenced in doc comment doesn't exist",
                    field_ref
                ),
            )
            .to_compile_error();
        }
    }

    if field_refs.is_empty() {
        quote! {
            write!(f, #doc_str)
        }
    } else {
        let fmt_str = build_fmt_str(doc_str);
        let fmt_args = field_refs.iter().map(|name| {
            let ident = Ident::new(name, proc_macro2::Span::call_site());
            quote! { self.#ident }
        });

        quote! {
            write!(f, #fmt_str, #(#fmt_args),*)
        }
    }
}

fn generate_enum_display(
    enum_name: &Ident,
    doc_str: &str,
    data_enum: &syn::DataEnum,
) -> proc_macro2::TokenStream {
    let has_variant_docs = data_enum
        .variants
        .iter()
        .any(|v| !extract_doc_comments(&v.attrs).is_empty());

    if !has_variant_docs {
        return quote! {
            write!(f, #doc_str)
        };
    }

    let match_arms = data_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_doc = extract_doc_comments(&variant.attrs);

        let display_str = if variant_doc.is_empty() {
            format!("{}::{}", enum_name, variant_name)
        } else {
            variant_doc
        };

        match &variant.fields {
            Fields::Unit => {
                quote! {
                    Self::#variant_name => write!(f, #display_str)
                }
            }
            Fields::Unnamed(fields) => {
                let field_bindings = (0..fields.unnamed.len())
                    .map(|i| Ident::new(&format!("_{}", i), proc_macro2::Span::call_site()))
                    .collect::<Vec<_>>();

                quote! {
                    Self::#variant_name(#(#field_bindings),*) => write!(f, #display_str)
                }
            }
            Fields::Named(fields) => {
                let field_names = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect::<Vec<_>>();

                quote! {
                    Self::#variant_name { #(#field_names),* } => write!(f, #display_str)
                }
            }
        }
    });

    quote! {
        match self {
            #(#match_arms,)*
        }
    }
}

fn extract_doc_comments(attrs: &[syn::Attribute]) -> String {
    let mut doc_string = String::new();

    for attr in attrs {
        if attr.path().is_ident("doc")
            && let Meta::NameValue(meta) = &attr.meta
            && let syn::Expr::Lit(expr_lit) = &meta.value
            && let syn::Lit::Str(lit_str) = &expr_lit.lit
        {
            let content = lit_str.value();
            let trimmed = content.strip_prefix(' ').unwrap_or(&content);

            if !doc_string.is_empty() {
                doc_string.push('\n');
            }

            doc_string.push_str(trimmed);
        }
    }

    doc_string
}

fn find_field_refs(doc_str: &str) -> Vec<String> {
    let re = regex::Regex::new(r"\{(\w+)\}").unwrap();
    re.captures_iter(doc_str)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn build_fmt_str(doc_str: &str) -> String {
    let re = regex::Regex::new(r"\{(\w+)\}").unwrap();
    re.replace_all(doc_str, "{}").to_string()
}
