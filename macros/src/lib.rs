use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, Path, PathArguments, Token};

#[proc_macro_derive(Wrap, attributes(parent))]
pub fn derive_wrap(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    let wrap_impl = input.attrs.iter().find_map(|attr| {
        if attr.path().get_ident().is_some_and(|i| i == "parent") {
            let path = attr.parse_args::<Path>().unwrap();
            let mut typath = path.clone();
            typath.segments.pop();
            typath.segments.pop_punct();
            let variant = &path.segments.last().unwrap().ident;
            Some(quote! {
                impl #name {
                    pub fn wrap(self) -> #typath {
                        #typath::#variant(self)
                    }
                }
            })
        } else {
            None
        }
    }).unwrap();

    wrap_impl.into()
}

#[proc_macro_derive(CharacterStateContainer)]
pub fn derive_character_state_container(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    let mut on_enter_variants = vec![];
    let mut on_exit_variants = vec![];
    let mut pre_tick_variants = vec![];
    let mut tick_variants = vec![];
    let mut priority_variants = vec![];
    let mut child_wrap_impl = vec![];

    match input.data {
        syn::Data::Struct(_) => todo!(),
        syn::Data::Enum(data) => for variant in data.variants {
            let delegate = variant.attrs.iter().find(|attr| attr.path().get_ident().is_some_and(|x| x == "delegate")).is_some();
            match &variant.fields {
                syn::Fields::Named(_) => todo!(),
                syn::Fields::Unnamed(fields) => {
                    let ty = &fields.unnamed.first().unwrap().ty;

                    let variant_name = &variant.ident;
                    
                    on_enter_variants.push(quote! {
                        Self::#variant_name(inner) => inner.on_enter(player)
                    });

                    on_exit_variants.push(quote! {
                        Self::#variant_name(inner) => inner.on_exit(player)
                    });

                    tick_variants.push(quote! {
                        Self::#variant_name(inner) => inner.tick(frame, player)
                    });

                    pre_tick_variants.push(quote! {
                        Self::#variant_name(inner) => inner.pre_tick(frame, player)
                    });

                    priority_variants.push(quote! {
                        Self::#variant_name(inner) => inner.priority()
                    });

                    child_wrap_impl.push(quote! {
                        impl #ty {
                            pub fn wrap(self) -> #name {
                                #name::#variant_name(self)
                            }
                        }
                    })
                },
                syn::Fields::Unit => todo!(),
            }
        },
        syn::Data::Union(_) => todo!(),
    }

    quote! {
        impl State for #name {
            fn on_enter(&mut self, player: PlayerSide) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
                match self {
                    #(#on_enter_variants,)*
                }
            }

            fn on_exit(&mut self, player: PlayerSide) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
                match self {
                    #(#on_exit_variants,)*
                }
            }

            fn pre_tick(&mut self, frame: Frame, player: PlayerSide) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
                match self {
                    #(#pre_tick_variants,)*
                }
            }
        
            fn tick(&mut self, frame: Frame, player: PlayerSide) -> Option<Box<dyn FnOnce(&mut GameState, &GameInfo)>> {
                match self {
                    #(#tick_variants,)*
                }
            }

            fn priority(&self) -> usize {
                match self {
                    #(#priority_variants,)*
                }
            }
        }

        #(#child_wrap_impl)*
    }.into()
}
