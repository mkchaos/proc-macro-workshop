use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Result};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::Item);
    match &item {
        syn::Item::Enum(item_enum) => {
            if let Err(e) = is_enum_sorted(item_enum) {
                let mut ts = e.to_compile_error();
                ts.extend(quote!(#item_enum));
                return ts.into();
            }
        }
        _ => {
            let args2 = proc_macro2::TokenStream::from(args);
            let err = syn::Error::new_spanned(args2, "expected enum or match expression");
            // err.extend(quote!(#item));
            return err.to_compile_error().into();
        }
    }

    quote! {
        #item
    }
    .into()
}

fn is_enum_sorted(item: &syn::ItemEnum) -> Result<()> {
    let mut str_vec: Vec<String> = Vec::new();
    for v in item.variants.iter() {
        let sid = v.ident.to_string();
        for s in str_vec.iter() {
            if s.gt(&sid) {
                let emsg = format!("{} should sort before {}", sid, s);
                let e = syn::Error::new(v.ident.span(), emsg);
                return Err(e);
            }
        }
        str_vec.push(sid);
    }
    Ok(())
}
