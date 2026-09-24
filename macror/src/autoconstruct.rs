use {
    proc_macro2::TokenStream,
    quote::{quote, quote_spanned},
    syn::{Data, DeriveInput, Error, Fields, Result, spanned::Spanned},
};

pub(crate) fn expand_auto_construct(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let vis = &input.vis;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unnamed(_) => {
                return Err(Error::new_spanned(
                    &input,
                    "AutoConstruct only supports structs with named fields",
                ));
            }
            Fields::Unit => {
                return Err(Error::new_spanned(
                    &input,
                    "AutoConstruct cannot be derived for unit structs",
                ));
            }
        },
        Data::Enum(data_enum) => {
            return Err(Error::new_spanned(
                data_enum.enum_token,
                "AutoConstruct cannot be derived for enums",
            ));
        }
        Data::Union(data_union) => {
            return Err(Error::new_spanned(
                data_union.union_token,
                "AutoConstruct cannot be derived for unions",
            ));
        }
    };

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let new_params = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        quote_spanned! { f.span() =>
            #field_name: #field_type
        }
    });

    let field_assignments = fields.iter().map(|f| {
        let field_name = &f.ident;
        quote_spanned! { f.span() =>
            #field_name
        }
    });

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            #[doc = "Create a new instance."]
            #[inline]
            #vis fn new(#(#new_params),*) -> Self {
                Self {
                    #(#field_assignments),*
                }
            }
        }
    })
}
