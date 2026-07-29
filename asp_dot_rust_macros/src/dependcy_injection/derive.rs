use proc_macro::TokenStream;

pub(crate) fn derive_di(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let struct_name = &ast.ident;

    let syn::Data::Struct(data) = &ast.data else {
        panic!("Only support struct");
    };
    let syn::Fields::Named(fields) = &data.fields else {
        panic!("Only support named fields");
    };

    let init_props = fields.named.iter().map(|f| {
        let prop_name = &f.ident;
        let prop_type = &f.ty;
        // eprintln!("prop_name: {:?}, prop_type: {:?}", prop_name, f.into_token_stream());
        quote::quote! {
            #prop_name: service_scope.get_service::<#prop_type>()
        }
    });

    quote::quote! {

        impl DependcyInjectableService for #struct_name {
            fn inject(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
                Self {
                    #(#init_props),*
                }
            }
        }
    }
    .into()
}
