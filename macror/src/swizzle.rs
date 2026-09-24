use {
    proc_macro2::{Span, TokenStream},
    quote::quote,
    syn::{Data, DeriveInput, Error, Fields, Ident, Result, Type},
};

pub(crate) fn expand_swizzle(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    let named_fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            other => {
                return Err(Error::new_spanned(
                    other,
                    "#[derive(Swizzle)] requires a struct with named fields",
                ));
            }
        },
        Data::Enum(e) => {
            return Err(Error::new_spanned(
                e.enum_token,
                "#[derive(Swizzle)] cannot be applied to enums",
            ));
        }
        Data::Union(u) => {
            return Err(Error::new_spanned(
                u.union_token,
                "#[derive(Swizzle)] cannot be applied to unions",
            ));
        }
    };

    let active: Vec<(&Ident, &Type)> = named_fields
        .iter()
        .map(|f| {
            let ident = f.ident.as_ref().expect("named field has ident");
            let ignored = has_swizzle_ignore(f)?;
            Ok((ident, &f.ty, ignored))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|(_, _, ignored)| !ignored)
        .map(|(id, ty, _)| (id, ty))
        .collect();

    if active.is_empty() {
        let (ig, tg, wc) = input.generics.split_for_impl();
        return Ok(quote! { impl #ig #struct_name #tg #wc {} });
    }

    let n = active.len();
    let mut methods = Vec::new();

    for length in 1..=n {
        for perm in permutations_with_repetition(&active, length) {
            let method_ident = Ident::new(
                &perm
                    .iter()
                    .map(|(id, _)| id.to_string())
                    .collect::<String>(),
                Span::call_site(),
            );

            let field_idents: Vec<&Ident> = perm.iter().map(|(id, _)| *id).collect();
            let field_types: Vec<&Type> = perm.iter().map(|(_, ty)| *ty).collect();

            let method = if length == 1 {
                let fid = field_idents[0];
                let fty = field_types[0];
                quote! {
                    #[inline(always)]
                    pub fn #method_ident(&self) -> &#fty {
                        &self.#fid
                    }
                }
            } else {
                quote! {
                    #[inline(always)]
                    pub fn #method_ident(&self) -> (#(&#field_types),*) {
                        (#(&self.#field_idents),*)
                    }
                }
            };

            methods.push(method);
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #struct_name #ty_generics #where_clause {
            #(#methods)*
        }
    })
}

fn has_swizzle_ignore(field: &syn::Field) -> Result<bool> {
    for attr in &field.attrs {
        if !attr.path().is_ident("swizzle") {
            continue;
        }
        let inner: Ident = attr
            .parse_args()
            .map_err(|_| Error::new_spanned(attr, "expected `#[swizzle(ignore)]`"))?;
        if inner != "ignore" {
            return Err(Error::new_spanned(
                &inner,
                format!("unknown swizzle option `{inner}`; expected `ignore`"),
            ));
        }
        return Ok(true);
    }
    Ok(false)
}

fn permutations_with_repetition<T: Copy>(pool: &[T], length: usize) -> Vec<Vec<T>> {
    if length == 0 {
        return vec![vec![]];
    }
    let mut result = Vec::new();
    for item in pool.iter().copied() {
        for mut suffix in permutations_with_repetition(pool, length - 1) {
            suffix.insert(0, item);
            result.push(suffix);
        }
    }
    result
}
