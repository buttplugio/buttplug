extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;

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
    let gen = quote! {
        impl ButtplugMessage for #name {
            fn get_id(&self) -> u32 {
                self.id
            }

            fn set_id(&mut self, id: u32) {
                self.id = id;
            }

            fn as_union(self) -> ButtplugMessageUnion {
                ButtplugMessageUnion::#name(self)
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(ButtplugDeviceMessage)]
pub fn buttplug_device_message_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_buttplug_device_message_macro(&ast)
}

fn impl_buttplug_device_message_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl ButtplugMessage for #name {
            fn get_id(&self) -> u32 {
                self.id
            }

            fn set_id(&mut self, id: u32) {
                self.id = id;
            }

            fn as_union(self) -> ButtplugMessageUnion {
                ButtplugMessageUnion::#name(self)
            }
        }

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
