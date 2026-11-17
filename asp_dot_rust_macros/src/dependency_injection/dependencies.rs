use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Type;

use crate::utils::extract_type::{extract_wrapper_inner, type_name};

#[derive(Default)]
pub(super) struct Dependencies {
    inner_types: Vec<Type>,
    needs_configuration: bool,
}

impl Dependencies {
    pub(super) fn resolve(&mut self, ty: &Type) -> Option<TokenStream> {
        let wrapper = type_name(ty)?.to_string();
        let inner = extract_wrapper_inner(ty, &wrapper)?;
        let expression = match wrapper.as_str() {
            "Serv" => quote! { service_scope.get_service::<#inner>().await },
            "Cfg" | "CfgOption" => quote! { configuration_service.get::<#inner>().capture() },
            "CfgRequire" => quote! { configuration_service.get::<#inner>().capture().unwrap() },
            "CfgReload" => quote! { configuration_service.get::<#inner>() },
            _ => return None,
        };
        self.needs_configuration |= wrapper != "Serv";
        self.inner_types.push(inner);
        Some(expression)
    }

    pub(super) fn configuration_binding(&self, main_crate: &TokenStream) -> TokenStream {
        if self.needs_configuration {
            quote! {
                let configuration_service = service_scope.get_service::<#main_crate::services::configuration::ConfigurationService>().await;
            }
        } else {
            TokenStream::new()
        }
    }

    pub(super) fn register(&self, main_crate: &TokenStream, owner: &impl ToTokens) -> TokenStream {
        let owner_name = owner.to_token_stream().to_string();
        let inner_types = &self.inner_types;
        quote! {
            #main_crate::third_party::inventory::submit! {
                #main_crate::dependency_injection::DependencyEdge {
                    owner: std::any::TypeId::of::<#owner>,
                    owner_name: #owner_name,
                    dependencies: || vec![ #( (std::any::TypeId::of::<#inner_types>(), std::any::type_name::<#inner_types>()) ),* ],
                }
            }
        }
    }
}
