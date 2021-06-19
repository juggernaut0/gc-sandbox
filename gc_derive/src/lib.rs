use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Fields, GenericArgument, parse_macro_input, PathArguments, Type, TypePath};
use syn::spanned::Spanned;

#[proc_macro_derive(GcNew)]
pub fn derive_gc_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

    // Generate a function to create the type
    let function = gc_new_fn(&input.data);

    let expanded = quote! {
        // The generated impl.
        // TODO generics
        impl #name {
            #function
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

fn gc_new_fn(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let params = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        match ty {
                            Type::Path(ref path) if type_is_gc_ptr(path) => {
                                let inner = match path.path.segments.last().unwrap().arguments {
                                    PathArguments::AngleBracketed(ref params) => {
                                        params.args.iter()
                                            .filter_map(|ga| {
                                                match ga {
                                                    GenericArgument::Type(ref inner) => Some(inner),
                                                    _ => None
                                                }
                                            })
                                            .next()
                                            .unwrap()
                                    }
                                    _ => unimplemented!()
                                };
                                quote_spanned! {f.span()=>
                                    #name: gc::GcBor<'ctx, 'gc, #inner>
                                }
                            }
                            _ => {
                                quote_spanned! {f.span()=>
                                    #name: #ty
                                }
                            }
                        }
                    });

                    let assigns = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        match ty {
                            Type::Path(ref path) if type_is_gc_ptr(path) => {
                                quote_spanned! {f.span()=>
                                    #name: unsafe { GcPtr::from_bor(#name) }
                                }
                            }
                            _ => {
                                quote_spanned! {f.span()=>
                                    #name
                                }
                            }
                        }
                    });

                    quote! {
                        fn gc_new<'ctx, 'gc>(__gc_ctx: &'ctx gc::GcContext<'gc> #(, #params)*) -> gc::GcBor<'ctx, 'gc, Self> {
                            __gc_ctx.allocate(Self { #(#assigns ,)* })
                        }
                    }
                }
                _ => unimplemented!()
            }
        }
        _ => unimplemented!()
    }
}

fn type_is_gc_ptr(path: &TypePath) -> bool {
    let last_segment = path.path.segments.last().unwrap();
    last_segment.ident == "GcPtr"
}
