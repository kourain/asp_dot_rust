use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Type;

use crate::utils::extract_type::{extract_wrapper_inner, type_name};

#[derive(Default)]
pub(super) struct Dependencies {
    services: Vec<Type>,
    configurations: Vec<(Type, bool)>,
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
        if wrapper == "Serv" {
            self.services.push(inner);
        } else {
            self.configurations.push((inner, wrapper == "CfgRequire"));
        }
        Some(expression)
    }

    pub(super) fn configuration_binding(&self, main_crate: &TokenStream) -> TokenStream {
        if !self.configurations.is_empty() {
            quote! {
                let configuration_service = service_scope.get_service::<#main_crate::services::configuration::ConfigurationService>().await;
            }
        } else {
            TokenStream::new()
        }
    }

    fn dependency_list(&self, main_crate: &TokenStream) -> TokenStream {
        let services = &self.services;
        let configurations = self.configurations.iter().map(|(ty, require_value)| {
            quote! { #main_crate::dependency_injection::Dependency::configuration::<#ty, #require_value>() }
        });
        quote! {
            #( #main_crate::dependency_injection::Dependency::service::<#services>(), )*
            #( #configurations, )*
        }
    }

    pub(super) fn check_registrations(&self, owner: &impl ToTokens) -> TokenStream {
        let services = &self.services;
        let configurations = self.configurations.iter().map(|(ty, _)| ty);
        quote! {
            #(
                assert!(
                    service_scope.contains_service::<#services>(),
                    "Invalid dependency: {} -> service {} is not registered",
                    std::any::type_name::<#owner>(), std::any::type_name::<#services>()
                );
            )*
            #(
                assert!(
                    configuration_service.contains::<#configurations>(),
                    "Invalid dependency: {} -> configuration {} is not registered",
                    std::any::type_name::<#owner>(), std::any::type_name::<#configurations>()
                );
            )*
        }
    }

    pub(super) fn register(&self, main_crate: &TokenStream, owner: &impl ToTokens) -> TokenStream {
        if self.services.is_empty() && self.configurations.is_empty() {
            return TokenStream::new();
        }
        let dependencies = self.dependency_list(main_crate);
        quote! {
            #main_crate::third_party::inventory::submit! {
                #main_crate::dependency_injection::ServiceDependencies {
                    owner: std::any::TypeId::of::<#owner>,
                    owner_name: std::any::type_name::<#owner>,
                    dependencies: || vec![#dependencies],
                }
            }
        }
    }
}
