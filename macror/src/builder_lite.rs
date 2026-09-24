use {
    proc_macro2::TokenStream,
    quote::{quote, quote_spanned},
    syn::{Data, DeriveInput, Error, Field, Fields, Ident, Result, Visibility, spanned::Spanned},
};

pub(crate) fn expand_builder_lite(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unnamed(_) => {
                return Err(Error::new_spanned(
                    &input,
                    "BuilderLite only supports structs with named fields",
                ));
            }
            Fields::Unit => {
                return Err(Error::new_spanned(
                    &input,
                    "BuilderLite cannot be derived for unit structs",
                ));
            }
        },
        Data::Enum(data_enum) => {
            return Err(Error::new_spanned(
                data_enum.enum_token,
                "BuilderLite cannot be derived for enums",
            ));
        }
        Data::Union(data_union) => {
            return Err(Error::new_spanned(
                data_union.union_token,
                "BuilderLite cannot be derived for unions",
            ));
        }
    };

    if !input.generics.params.is_empty() {
        let generics = &input.generics;
        return Err(Error::new_spanned(
            generics,
            "BuilderLite does not currently support generics",
        ));
    }

    let mut builder_methods = Vec::new();
    let mut errors = Vec::new();

    for field in fields {
        match process_field(field) {
            Ok(Some(method)) => builder_methods.push(method),
            Ok(None) => continue,
            Err(err) => errors.push(err),
        }
    }

    if !errors.is_empty() {
        let mut combined_error = errors.clone().into_iter().next().unwrap();
        for err in errors {
            combined_error.combine(err);
        }

        return Err(combined_error);
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            #(#builder_methods)*
        }
    })
}

pub(crate) struct FieldConfig {
    skip: bool,
    custom_name: Option<String>,
}

pub(crate) fn process_field(field: &Field) -> Result<Option<TokenStream>> {
    let field_name = field.ident.as_ref().unwrap();
    let field_type = &field.ty;
    let field_vis = &field.vis;
    let cfg = parse_builder_attrs(&field.attrs)?;

    if cfg.skip {
        return Ok(None);
    }

    let method_name = if let Some(custom) = cfg.custom_name {
        Ident::new(&custom, field_name.span())
    } else {
        Ident::new(&format!("with_{}", field_name), field_name.span())
    };

    let doc_comment = format!("Set the `{}` field.", field_name);

    validate_field_visibility(field_vis, field_name)?;

    let method = quote_spanned! { field.span() =>
        #[allow(unused)]
        #[doc = #doc_comment]
        #[inline]
        #field_vis fn #method_name(mut self, value: #field_type) -> Self {
            self.#field_name = value;
            self
        }
    };

    Ok(Some(method))
}

pub(crate) fn parse_builder_attrs(attrs: &[syn::Attribute]) -> Result<FieldConfig> {
    let mut config = FieldConfig {
        skip: false,
        custom_name: None,
    };

    for attr in attrs {
        if !attr.path().is_ident("builder") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                if config.skip {
                    return Err(meta.error("duplicate `skip` attribute"));
                }

                config.skip = true;
                Ok(())
            } else if meta.path.is_ident("rename") {
                if config.custom_name.is_some() {
                    return Err(meta.error("duplicate `rename` attribute"));
                }

                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                let name = s.value();

                if name.is_empty() {
                    return Err(Error::new_spanned(s, "method name cannot be empty"));
                }

                if !name.chars().next().unwrap().is_alphabetic() && !name.starts_with('_') {
                    return Err(Error::new_spanned(
                        s,
                        "method name must start with a letter or underscore",
                    ));
                }

                config.custom_name = Some(name);
                Ok(())
            } else {
                Err(meta.error(format!(
                    "unknown builder attribute `{}`. \
                            Valid attributes are: `skip`, `rename = \"...\"`",
                    meta.path
                        .get_ident()
                        .map(|i| i.to_string())
                        .unwrap_or_default()
                )))
            }
        })?;
    }

    Ok(config)
}

pub(crate) fn validate_field_visibility(vis: &Visibility, _field_name: &Ident) -> Result<()> {
    if let Visibility::Inherited = vis {}

    Ok(())
}
