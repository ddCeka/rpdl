use syn::DeriveInput;

#[cfg(feature = "builder_lite")]
mod builder_lite;

#[cfg(feature = "doc_display")]
mod doc_display;

#[cfg(feature = "minmax")]
mod minmax;

#[cfg(feature = "ctrlc")]
mod ctrlc;

#[cfg(feature = "eyre")]
mod eyre;

#[cfg(feature = "builder")]
mod builder;

#[cfg(feature = "extends")]
mod extends;

#[cfg(feature = "main")]
mod _main;

#[cfg(feature = "autoconstruct")]
mod autoconstruct;

#[cfg(feature = "math")]
mod math;

#[cfg(feature = "swizzle")]
mod swizzle;

#[cfg(feature = "builder_lite")]
#[proc_macro_derive(BuilderLite, attributes(builder))]
/// Automatically implements the builder lite pattern for a struct
pub fn derive_builder_lite(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    builder_lite::expand_builder_lite(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[cfg(feature = "doc_display")]
#[proc_macro_derive(DocDisplay)]
/// Automatically generates an `std::fmt::Display` implementation
/// for structs and enums based on the documentation comments of the
/// given struct/enum.
pub fn derive_doc_display(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    doc_display::expand_doc_display(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[cfg(feature = "swizzle")]
#[proc_macro_derive(Swizzle, attributes(swizzle))]
pub fn derive_swizzle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    swizzle::expand_swizzle(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[cfg(feature = "minmax")]
#[proc_macro_attribute]
/// Validates integer function parameters against minimum and maximum bounds at compile-time.
pub fn minmax(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    minmax::expand_minmax(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[cfg(feature = "math")]
#[proc_macro_attribute]
/// Replaces unchecked math with checked math
pub fn math(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    math::expand_math(attr, item)
}

#[cfg(feature = "ctrlc")]
#[proc_macro_attribute]
/// Registers a function as a Ctrl-C signal handler using the `ctrlc` crate.
///
/// The annotated function will be called when a Ctrl-C signal (SIGINT) is received.
/// The function must have no parameters and must return `()` or have no return type.
pub fn ctrlc(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    ctrlc::expand_ctrlc(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[cfg(feature = "eyre")]
#[proc_macro_attribute]
/// Automatically sets up color-eyre error handling for the main function.
pub fn eyre(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    eyre::expand_eyre(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[cfg(feature = "autoconstruct")]
#[proc_macro_derive(Construct)]
/// Automatically generates a `new` method for unit structs
pub fn autoconstruct(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    autoconstruct::expand_auto_construct(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[cfg(feature = "builder")]
#[proc_macro_derive(Builder)]
/// Sets up a full builder implementation for a struct.
pub fn derive_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    builder::expand_builder(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[cfg(feature = "extends")]
#[proc_macro_attribute]
/// Takes a given function and moves it into a custom `impl` block for a given struct/enum.
///
/// Supports both `&self` methods and associated functions (non-`&self` methods).
pub fn extends(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let config = syn::parse_macro_input!(attr as extends::ExtendsConfig);
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    extends::expand_extends(config, input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[cfg(feature = "main")]
#[proc_macro_attribute]
/// Allows using another name for the main function.
pub fn main(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let config = syn::parse_macro_input!(attr as _main::MainConfig);
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    _main::expand_main(config, input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}