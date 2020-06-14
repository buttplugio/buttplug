extern crate proc_macro;

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

#[proc_macro_derive(TryFromButtplugClientMessage)]
pub fn try_from_buttplug_client_message_derive(input: TokenStream) -> TokenStream {
  // Construct a representation of Rust code as a syntax tree
  // that we can manipulate
  let ast = syn::parse(input).unwrap();

  impl_try_from_buttplug_client_message_derive_macro(&ast)
}

fn impl_try_from_buttplug_client_message_derive_macro(ast: &syn::DeriveInput) -> TokenStream {
  let name = &ast.ident;
  if let syn::Data::Enum(ref e) = ast.data {
    let idents: Vec<_> = e.variants.iter().map(|x| x.ident.clone()).collect();
    let gen = quote! {
        impl TryFrom<ButtplugClientMessage> for #name {
            type Error = &'static str;

            fn try_from(msg: ButtplugClientMessage) -> Result<Self, &'static str> {
                match msg {
                    #( ButtplugClientMessage::#idents(msg) => Ok(#name::#idents(msg)),)*
                    _ => Err("ButtplugClientMessage cannot be converted to #name")
                }
            }
        }

        impl From<#name> for ButtplugClientMessage {
            fn from(msg: #name) -> ButtplugClientMessage {
                match msg {
                    #( #name::#idents(msg) => ButtplugClientMessage::#idents(msg),)*
                }
            }
        }
    };
    gen.into()
  } else {
    panic!("TryFromButtplugClientMessage only works on structs");
  }
}

#[proc_macro_derive(TryFromButtplugServerMessage)]
pub fn try_from_buttplug_out_message_derive(input: TokenStream) -> TokenStream {
  // Construct a representation of Rust code as a syntax tree
  // that we can manipulate
  let ast = syn::parse(input).unwrap();

  impl_try_from_buttplug_server_message_derive_macro(&ast)
}

fn impl_try_from_buttplug_server_message_derive_macro(ast: &syn::DeriveInput) -> TokenStream {
  let name = &ast.ident;
  if let syn::Data::Enum(ref e) = ast.data {
    let idents: Vec<_> = e.variants.iter().map(|x| x.ident.clone()).collect();
    let gen = quote! {
        impl TryFrom<ButtplugServerMessage> for #name {
            type Error = ButtplugMessageError;

            fn try_from(msg: ButtplugServerMessage) -> Result<Self, ButtplugMessageError> {
                match msg {
                    #( ButtplugServerMessage::#idents(msg) => Ok(#name::#idents(msg.into())),)*
                    _ => Err(ButtplugMessageError::new("ButtplugServerMessage cannot be converted to #name"))
                }
            }
        }
    };
    gen.into()
  } else {
    panic!("TryFromButtplugServerMessage only works on structs");
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
    panic!("FromButtplugMessageUnion only works on structs");
  }
}

#[proc_macro_derive(ButtplugClientMessageType)]
pub fn buttplug_client_message_type_derive(input: TokenStream) -> TokenStream {
  // Construct a representation of Rust code as a syntax tree
  // that we can manipulate
  let ast = syn::parse(input).unwrap();

  // Build the trait implementation
  impl_buttplug_client_message_type_macro(&ast)
}

fn impl_buttplug_client_message_type_macro(ast: &syn::DeriveInput) -> TokenStream {
  let name = &ast.ident;
  let gen = quote! {
      impl ButtplugClientMessageType for #name {}
  };
  gen.into()
}

#[proc_macro_derive(ButtplugServerMessageType)]
pub fn buttplug_server_message_type_derive(input: TokenStream) -> TokenStream {
  // Construct a representation of Rust code as a syntax tree
  // that we can manipulate
  let ast = syn::parse(input).unwrap();

  // Build the trait implementation
  impl_buttplug_server_message_type_macro(&ast)
}

fn impl_buttplug_server_message_type_macro(ast: &syn::DeriveInput) -> TokenStream {
  let name = &ast.ident;
  let gen = quote! {
      impl ButtplugServerMessageType for #name {}
  };
  gen.into()
}

#[proc_macro_derive(ButtplugProtocol)]
pub fn buttplug_protocol_derive(input: TokenStream) -> TokenStream {
  // Construct a representation of Rust code as a syntax tree
  // that we can manipulate
  let ast = syn::parse(input).unwrap();

  // Build the trait implementation
  impl_buttplug_protocol_macro(&ast)
}

fn impl_buttplug_protocol_macro(ast: &syn::DeriveInput) -> TokenStream {
  let name = &ast.ident;
  let gen = quote! {
      impl ButtplugProtocol for #name {}
  };
  gen.into()
}

#[proc_macro_derive(ButtplugProtocolProperties)]
pub fn buttplug_protocol_properties_derive(input: TokenStream) -> TokenStream {
  // Construct a representation of Rust code as a syntax tree
  // that we can manipulate
  let ast = syn::parse(input).unwrap();

  // Build the trait implementation
  impl_buttplug_protocol_properties_macro(&ast)
}

fn impl_buttplug_protocol_properties_macro(ast: &syn::DeriveInput) -> TokenStream {
  let name = &ast.ident;
  let gen = quote! {
      impl ButtplugProtocolProperties for #name {
          fn name(&self) -> &str {
            &self.name
          }

          fn message_attributes(&self) -> MessageAttributesMap {
            self.message_attributes.clone()
          }

          fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion> {
            self.stop_commands.clone()
          }
        }
  };
  gen.into()
}

#[proc_macro_derive(ButtplugProtocolCreator)]
pub fn buttplug_protocol_creator_derive(input: TokenStream) -> TokenStream {
  // Construct a representation of Rust code as a syntax tree
  // that we can manipulate
  let ast = syn::parse(input).unwrap();

  // Build the trait implementation
  impl_buttplug_protocol_creator_macro(&ast)
}

fn impl_buttplug_protocol_creator_macro(ast: &syn::DeriveInput) -> TokenStream {
  let name = &ast.ident;
  let gen = quote! {
       impl ButtplugProtocolCreator for #name {
        fn new_protocol(name: &str, attrs: MessageAttributesMap) -> Box<dyn ButtplugProtocol> {
          Box::new(Self::new(name, attrs))
        }
      }
  };
  gen.into()
}
