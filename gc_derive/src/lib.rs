use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Fields, GenericArgument, parse_macro_input, PathArguments, Type, TypePath, Generics, GenericParam, parse_quote, Index};
use syn::spanned::Spanned;

#[proc_macro_derive(GcNew)]
pub fn derive_gc_new(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Generate a function to create the type
    let function = gc_new_fn(&input.data);

    let expanded = quote! {
        // The generated impl.
        impl #impl_generics #name #ty_generics #where_clause {
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

#[proc_macro_derive(Trace)]
pub fn derive_trace(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

    // Add a bound `T: Trace` to every type parameter T.
    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Generate an expression to sum up the heap size of each field.
    let fn_body = trace_fn_body(&input.data);

    let expanded = quote! {
        // The generated impl.
        unsafe impl #impl_generics gc::Trace for #name #ty_generics #where_clause {
            fn trace(&self, tracer: &mut gc::Tracer) {
                #fn_body
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

// Add a bound `T: Trace` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(gc::Trace));
        }
    }
    generics
}

fn trace_fn_body(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! {f.span()=>
                            self.#name.trace(tracer);
                        }
                    });
                    quote! {
                        #(#recurse)*
                    }
                }
                Fields::Unnamed(ref fields) => {
                    let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                        let index = Index::from(i);
                        quote_spanned! {f.span()=>
                            self.#index.trace(tracer);
                        }
                    });
                    quote! {
                        #(#recurse)*
                    }
                }
                Fields::Unit => {
                    quote!(())
                }
            }
        }
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
