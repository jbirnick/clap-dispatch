//! Ergonomic way to dispatch CLI subcommands.
//!
//! Useful when your CLI defines subcommands, all of which should do the same kind of action, just in a different way. \
//! I.e., when the subcommands are variants of running a certain action.
//!
//! It becomes especially useful when you have nested subcommands.
//! In this case you can dispatch all the way down to the leaves of your command tree!
//!
//! # Example
//!
//! Suppose you implement a CLI for sorting numbers. \
//! You have two algorithms, Quick Sort and Merge Sort, and they have their own subcommand, respectively.
//! ```
//! #[derive(Parser)]
//! enum Cli {
//!     Quick(QuickArgs),
//!     Merge(MergeArgs),
//! }
//! ```
//! Now the point is, both algorithms will essentially just implement some `sort(...)` function with the same signature.
//! You could model this as both `QuickArgs` and `MergeArgs` implementing a function like:
//! ```
//! fn sort(self, nums: Vec<i32>) -> Vec<i32>
//! ```
//! (The `self` is there so that they can make use of the special arguments passed for the respective algorithm.) \
//! So you could put such a function into a trait, and then implement the trait for both `QuickArgs` and `MergeArgs`.
//!
//! The annoying part is, to dispatch the `sort(...)` function, you then have to do a `match` over your `Cli` enum and call `sort(...)` on every variant.
//! That's boilerplate.
//!
//! This crate is doing the boilerplate for you.
//!
//! In this case, you would do the following macro invocation:
//! ```
//! #[derive(Parser)]
//! #[clap_dispatch(fn sort(self, nums: Vec<i32>) -> Vec<i32>)] // macro call
//! enum Cli {
//!     Quick(QuickArgs),
//!     Merge(MergeArgs),
//! }
//! ```
//! This defines a trait `Sort` which contains the `sort(...)` function, and it also implements `Sort` for `Cli`, where the implementation just dispatches to the variants.
//! So what's left to you is only:
//! - implement `Sort` for `QuickArgs` and `MergeArgs` (i.e. implement the algorithms)
//! - call `cli.sort(...)`
//!
//! # Usage
//!
//! A minimal explanation is in the definition of the [macro@clap_dispatch] macro.
//!
//! The full code for the above example can be found in the `example/` folder. ([latest GitHub version](https://github.com/jbirnick/clap-dispatch/tree/main/example))

use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, ItemEnum, Signature};

/// The main macro.
///
/// Needs to be attached to an `enum` and given a function signature as an attribute.
/// ```
/// #[clap_dispatch(fn run(self))]
/// enum MyCommand {
///   Foo(FooArgs)
///   Bar(BarArgs)
/// }
/// ```
///
/// It does two things:
///
/// 1. It creates a new trait, named like the provided function transformed to UpperCamelCase.
///    The trait will contain only one function, which has exactly the provided signature.
///
///    In this case it will generate:
///    ```
///    trait Run {
///        fn run(self);
///    }
///    ```
///
/// 2. It implements the trait for the enum.
///    The implementation is just to dispatch onto the different variants.
///    **This means the fields of all the enum variants need to implement the generated trait from above.**
///    It's your job to make those implementations by hand.
///
///    In this case it will generate:
///    ```
///    impl Run for MyCommand {
///       fn run(self) {
///           match self {
///               Self::Foo(args) => args.run(),
///               Self::Bar(args) => args.run(),
///           }
///       }
///    }
///    ```
///
#[proc_macro_attribute]
pub fn clap_dispatch(attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let generated =
        clap_dispatch_gen(&attr, &item).unwrap_or_else(|error| error.to_compile_error().into());
    item.extend(generated);
    item
}

fn clap_dispatch_gen(attr: &TokenStream, item: &TokenStream) -> Result<TokenStream, syn::Error> {
    // parse the enum and the attribute
    let item_enum: ItemEnum = syn::parse(item.clone())?;
    let signature: Signature = syn::parse(attr.clone())?;

    // generate new things which should be appended after the enum
    generate(item_enum, signature)
}

// generates both:
// 1. the new trait whose only function is given by the provided signature
// 2. the implementation of this trait for the enum
fn generate(
    // the enum on which the attribute macro was placed
    item_enum: ItemEnum,
    // the function signature that was provided with the attribute macro
    signature: Signature,
) -> Result<TokenStream, syn::Error> {
    // make sure the user provided everything in the correct form
    validity_checks(&item_enum, &signature)?;

    // relevant identifiers
    let enum_ident = item_enum.ident;
    let signature_ident = &signature.ident;
    let trait_ident = upper_camel_case(signature_ident);

    // the arguments which need to be passed to the function, except `self`
    let call_args = signature.inputs.iter().skip(1).map(|fn_arg| {
        if let syn::FnArg::Typed(pat_type) = fn_arg {
            &pat_type.pat
        } else {
            // all functions arguments except the first one should be FnArg::Typed (not FnArg::Receiver)
            unreachable!()
        }
    });

    // the match arms for the implementation of the trait
    let match_arms = item_enum.variants.into_iter().map(|variant| {
        let variant_ident = variant.ident;
        let call_args = call_args.clone();

        quote! {
            Self::#variant_ident(args) => self::#trait_ident::#signature_ident(args, #(#call_args),*),
        }
    });

    // the final generated code
    let generated = quote! {
        trait #trait_ident {
            #signature;
        }

        impl #trait_ident for #enum_ident {
            #signature {
                match self {
                    #(#match_arms)*
                }
            }
        }
    };

    Ok(generated.into())
}

fn upper_camel_case(ident: &Ident) -> Ident {
    let new_ident = ident.to_string().to_upper_camel_case();
    Ident::new(&new_ident, Span::call_site())
}

fn validity_checks(item_enum: &ItemEnum, signature: &Signature) -> Result<(), syn::Error> {
    // make sure the enum doesn't use generics
    if item_enum.generics.lt_token.is_some() {
        return Err(syn::Error::new_spanned(
            &item_enum.generics,
            "generics are not yet supported by clap-dispatch",
        ));
    }

    // make sure signature has no generics
    if signature.generics.lt_token.is_some() {
        return Err(syn::Error::new_spanned(
            &signature.generics,
            "generics are not yet supported by clap-dispatch",
        ));
    }

    // make sure signature has no variadic
    if signature.variadic.is_some() {
        return Err(syn::Error::new_spanned(
            &signature.variadic,
            "variadics are not yet supported by clap-dispatch",
        ));
    }

    // make sure first argument of signature is some form of `self`
    match signature.inputs.first() {
        Some(fn_arg) => {
            if !matches!(fn_arg, syn::FnArg::Receiver(_)) {
                return Err(syn::Error::new_spanned(
                    fn_arg,
                    "first argument of function must be `self` or `&self` or `&mut self`",
                ));
            }
        }
        None => {
            return Err(syn::Error::new_spanned(
                &signature.inputs,
                "function needs at least a `self` argument (or `&self` or `&mut self`)",
            ))
        }
    }

    // make sure the enum variants have exactly one unnamed field
    for variant in item_enum.variants.iter() {
        match &variant.fields {
            syn::Fields::Named(fields_named) => {
                return Err(syn::Error::new_spanned(
                    fields_named,
                    "must have unnamed field, not named",
                ));
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                if fields_unnamed.unnamed.len() != 1 {
                    return Err(syn::Error::new_spanned(
                        fields_unnamed,
                        "number of unnamed fields must be exactly one",
                    ));
                }
            }
            syn::Fields::Unit => {
                return Err(syn::Error::new_spanned(
                    &variant.ident,
                    "variant must have an unnamed field",
                ));
            }
        };
    }

    Ok(())
}
