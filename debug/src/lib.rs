use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;
use quote::quote;
use syn::{parse_macro_input, parse_quote, spanned::Spanned, DeriveInput, Field, Result};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match expand(&ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn get_debug_attr(f: &Field) -> Option<Result<String>> {
    for attr in &f.attrs {
        let meta = match attr.parse_meta() {
            Err(e) => return Some(Err(e)),
            Ok(meta) => meta,
        };
        if let syn::Meta::NameValue(nv) = &meta {
            if nv.path.get_ident().unwrap() == "debug" {
                if let syn::Lit::Str(ls) = &nv.lit {
                    return Some(Ok(ls.value()));
                }
            }
        }
        return Some(Err(syn::Error::new(
            meta.span(),
            r#"expected `debug = "..."`"#,
        )));
    }
    None
}

fn add_bounds(g: &syn::Generics) -> syn::Generics {
    let mut g = g.clone();
    for p in &mut g.params {
        if let syn::GenericParam::Type(tp) = p {
            tp.bounds.push(parse_quote!(std::fmt::Debug));
        }
    }
    g
}

fn expand(ast: &DeriveInput) -> Result<TS2> {
    let name = &ast.ident;
    let sname = format!("{}", name);
    let g = add_bounds(&ast.generics);
    let (impl_generics, ty_generics, where_clause) = g.split_for_impl();
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = &ast.data
    {
        named
    } else {
        return Err(syn::Error::new(ast.span(), "Not a name struct"));
    };
    let debug_fields = fields.iter().map(|f| {
        let id = &f.ident;
        let sid = format!("{}", id.as_ref().unwrap());
        match get_debug_attr(f) {
            Some(Ok(s)) => {
                quote! {
                    .field(#sid, &format_args!(#s, self.#id))
                }
            }
            // Not process error
            _ => {
                quote! {
                    .field(#sid, &self.#id)
                }
            }
        }
    });
    Ok(quote! {
        impl #impl_generics std::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.debug_struct(#sname)
                   #(#debug_fields)*
                   .finish()
            }
        }
    })
}
