use {
    proc_macro2::TokenStream,
    quote::{quote, quote_spanned},
    syn::{Data, DeriveInput, Error, Fields, Ident, Result, spanned::Spanned},
};

pub(crate) fn expand_builder(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let vis = &input.vis;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unnamed(_) => {
                return Err(Error::new_spanned(
                    &input,
                    "Builder only supports structs with named fields",
                ));
            }
            Fields::Unit => {
                return Err(Error::new_spanned(
                    &input,
                    "Builder cannot be derived for unit structs",
                ));
            }
        },
        Data::Enum(data_enum) => {
            return Err(Error::new_spanned(
                data_enum.enum_token,
                "Builder cannot be derived for enums",
            ));
        }
        Data::Union(data_union) => {
            return Err(Error::new_spanned(
                data_union.union_token,
                "Builder cannot be derived for unions",
            ));
        }
    };

    if !input.generics.params.is_empty() {
        let generics = &input.generics;
        return Err(Error::new_spanned(
            generics,
            "Builder does not currently support generics",
        ));
    }

    let builder_name = Ident::new(&format!("{}Builder", name), name.span());

    let builder_fields = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        quote_spanned! { f.span() =>
            #field_name: ::core::option::Option<#field_type>
        }
    });

    let builder_methods = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_type = &f.ty;
        let field_vis = &f.vis;
        let doc_comment = format!("Set the `{}` field.", field_name);

        quote_spanned! { f.span() =>
            #[doc = #doc_comment]
            #[inline]
            #field_vis fn #field_name(mut self, value: #field_type) -> Self {
                self.#field_name = ::core::option::Option::Some(value);
                self
            }
        }
    });

    let build_fields = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();

        quote_spanned! { f.span() =>
            #field_name: self.#field_name.ok_or_else(|| {
                ::std::format!("field `{}` is required but was not set", #field_name_str)
            })?
        }
    });

    let default_fields_1 = fields.iter().map(|f| {
        let field_name = &f.ident;
        quote_spanned! { f.span() =>
            #field_name: ::core::option::Option::None
        }
    });

    let default_fields_2 = fields.iter().map(|f| {
        let field_name = &f.ident;
        quote_spanned! { f.span() =>
            #field_name: ::core::option::Option::None
        }
    });

    Ok(quote! {
        #[derive(Clone, Debug)]
        #vis struct #builder_name {
            #(#builder_fields,)*
        }

        impl #builder_name {
            #(#builder_methods)*

            #[doc = "Build the final instance, returning an error if any required fields are missing."]
            #vis fn build(self) -> ::std::result::Result<#name, ::std::string::String> {
                ::std::result::Result::Ok(#name {
                    #(#build_fields,)*
                })
            }
        }

        impl #name {
            #[doc = "Create a new builder instance."]
            #[inline]
            #vis fn builder() -> #builder_name {
                #builder_name {
                    #(#default_fields_1,)*
                }
            }
        }

        impl ::core::default::Default for #builder_name {
            fn default() -> Self {
                Self {
                    #(#default_fields_2,)*
                }
            }
        }
    })
}
