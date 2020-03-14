extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(ButtplugMessage)]
pub fn buttplug_message_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_buttplug_message_macro(&ast)
}

fn impl_buttplug_message_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    match ast.data {
        syn::Data::Enum(ref e) => {
            let idents = e.variants.iter().map(|x| x.ident.clone());
            let idents2 = idents.clone();
            let gen = quote! {
                impl ButtplugMessage for #name {
                    fn get_id(&self) -> u32 {
                        match self {
                            #( #name::#idents(ref msg) => msg.id,)*

                        }
                    }
                    fn set_id(&mut self, id: u32) {
                        match self {
                            #( #name::#idents2(ref mut msg) => msg.set_id(id),)*
                        }
                    }
                }
            };
            gen.into()
        }
        syn::Data::Struct(_) => {
            let gen = quote! {
                impl ButtplugMessage for #name {
                    fn get_id(&self) -> u32 {
                        self.id
                    }

                    fn set_id(&mut self, id: u32) {
                        self.id = id;
                    }
                }
            };
            gen.into()
        }
        _ => panic!("Derivation only works on structs and enums"),
    }
}

#[proc_macro_derive(ButtplugDeviceMessage)]
pub fn buttplug_device_message_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    let mut tokens = impl_buttplug_message_macro(&ast);
    tokens.extend(impl_buttplug_device_message_macro(&ast));
    tokens
}

fn impl_buttplug_device_message_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    match ast.data {
        syn::Data::Enum(ref e) => {
            let idents: Vec<_> = e.variants.iter().map(|x| x.ident.clone()).collect();
            let gen = quote! {
                impl ButtplugDeviceMessage for #name {
                    fn get_device_index(&self) -> u32 {
                        match self {
                            #( #name::#idents(ref msg) => msg.get_device_index(),)*

                        }
                    }
                    fn set_device_index(&mut self, id: u32) {
                        match self {
                            #( #name::#idents(ref mut msg) => msg.set_device_index(id),)*
                        }
                    }
                }
            };
            gen.into()
        }
        syn::Data::Struct(_) => {
            let gen = quote! {
                impl ButtplugDeviceMessage for #name {
                    fn get_device_index(&self) -> u32 {
                        self.device_index
                    }

                    fn set_device_index(&mut self, id: u32) {
                        self.device_index = id;
                    }
                }
            };
            gen.into()
        }
        _ => panic!("Derivation only works on structs and enums"),
    }
}

#[proc_macro_derive(TryFromButtplugMessageUnion)]
pub fn try_from_buttplug_message_union_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    impl_try_from_message_union_derive_macro(&ast)
}

fn impl_try_from_message_union_derive_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    if let syn::Data::Enum(ref e) = ast.data {
        let idents: Vec<_> = e.variants.iter().map(|x| x.ident.clone()).collect();
        let gen = quote! {
            impl TryFrom<ButtplugMessageUnion> for #name {
                type Error = &'static str;

                fn try_from(msg: ButtplugMessageUnion) -> Result<Self, &'static str> {
                    match msg {
                        #( ButtplugMessageUnion::#idents(msg) => Ok(#name::#idents(msg)),)*
                        _ => Err("ButtplugMessageUnion cannot be converted to #name")
                    }
                }
            }
        };
        gen.into()
    } else {
        panic!("TryFromButtplugMessageUnion only works on structs");
    }
}

#[proc_macro_derive(FromSpecificButtplugMessage)]
pub fn from_specific_buttplug_message_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    impl_from_specific_buttplug_message_derive_macro(&ast)
}

fn impl_from_specific_buttplug_message_derive_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    if let syn::Data::Enum(ref e) = ast.data {
        let idents: Vec<_> = e.variants.iter().map(|x| x.ident.clone()).collect();
        let gen = quote! {
            #(impl From<#idents> for #name {
                fn from(msg: #idents) -> #name {
                    #name::#idents(msg)
                }
            })*
        };
        gen.into()
    } else {
        panic!("TryFromButtplugMessageUnion only works on structs");
    }
}

#[proc_macro_derive(ToButtplugMessageUnion)]
pub fn to_buttplug_message_union_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_to_buttplug_message_union_macro(&ast)
}

fn impl_to_buttplug_message_union_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    match ast.data {
        syn::Data::Enum(ref e) => {
            let idents = e.variants.iter().map(|x| x.ident.clone());
            let gen = quote! {
                impl From<#name> for ButtplugMessageUnion {
                    fn from(msg: #name) -> ButtplugMessageUnion {
                        match msg {
                            #( #name::#idents(msg) => ButtplugMessageUnion::#idents(msg),)*
                        }
                    }
                }
            };
            gen.into()
        }
        syn::Data::Struct(_) => {
            let gen = quote! {
                impl From<#name> for ButtplugMessageUnion {
                    fn from(msg: #name) -> ButtplugMessageUnion {
                        ButtplugMessageUnion::#name(msg)
                    }
                }
            };
            gen.into()
        }
        _ => panic!("Derivation only works on structs and enums"),
    }
}

    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
}

    let name = &ast.ident;

    match ast.data {
        syn::Data::Enum(ref e) => {
            let idents = e.variants.iter().map(|x| x.ident.clone());
            let gen = quote! {
                        match self {
                        }
                    }
                }
            };
            gen.into()
        }
        syn::Data::Struct(_) => {
            let gen = quote! {
                    }
                }
            };
            gen.into()
        }
        _ => panic!("Derivation only works on structs and enums"),
    }
}