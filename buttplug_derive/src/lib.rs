extern crate quote;
extern crate proc_macro;
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
            fn id(&self) -> u32 {
                return self.id;
            }
        }
    };
    gen.into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
