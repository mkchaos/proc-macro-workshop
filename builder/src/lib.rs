use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;
use quote::{format_ident, quote};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, Result};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match expand(&ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct FieldAttr {
    ident: Option<syn::Ident>,
    each: Option<Result<String>>,
    ty: syn::Type,
    is_vec: bool,
    is_opt: bool,
    inner_type: Option<syn::Type>,
}

fn get_out_and_inner_type(ty: &syn::Type) -> Option<(String, syn::Type)> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        let seg = segments.last()?;
        let seg_str = seg.ident.to_string();
        if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            args,
            ..
        }) = &seg.arguments
        {
            if args.len() == 1 {
                let ga = args.first()?;
                if let syn::GenericArgument::Type(ty) = ga {
                    return Some((seg_str, ty.clone()));
                }
            }
        }
    }
    None
}

fn get_each_str(f: &syn::Field) -> Option<Result<String>> {
    for attr in &f.attrs {
        let meta = match attr.parse_meta() {
            Ok(meta) => meta,
            Err(e) => return Some(Err(e)),
        };
        if let syn::Meta::List(list) = &meta {
            if list.nested.len() == 1 {
                let a = list.nested.first().unwrap();
                if let syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) = a {
                    if nv.path.get_ident().unwrap() == "each" {
                        if let syn::Lit::Str(ls) = &nv.lit {
                            return Some(Ok(ls.value()));
                        }
                    }
                }
            }
        }
        return Some(Err(syn::Error::new(
            meta.span(),
            r#"expected `builder(each = "...")`"#,
        )));
    }
    None
}

fn get_field_type_attr(f: &syn::Field) -> FieldAttr {
    match get_out_and_inner_type(&f.ty) {
        Some((out, inner)) => FieldAttr {
            ident: f.ident.clone(),
            each: get_each_str(f),
            ty: f.ty.clone(),
            is_vec: out == "Vec",
            is_opt: out == "Option",
            inner_type: Some(inner),
        },
        None => FieldAttr {
            ident: f.ident.clone(),
            each: get_each_str(f),
            ty: f.ty.clone(),
            is_vec: false,
            is_opt: false,
            inner_type: None,
        },
    }
}

fn expand(ast: &DeriveInput) -> Result<TS2> {
    let name = &ast.ident;
    let builder_name = format_ident!("{}Builder", name);
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = &ast.data
    {
        named
    } else {
        return Err(syn::Error::new(ast.span(), "Not a name struct"));
    };
    let type_attrs: Vec<_> = fields.iter().map(|f| get_field_type_attr(f)).collect();
    for a in type_attrs.iter() {
        if a.ident.is_none() {
            return Err(syn::Error::new(a.ty.span(), "There are no name ident"));
        }
        if let Some(Err(e)) = &a.each {
            return Err(e.clone());
        }
    }
    let builder_declares = type_attrs.iter().map(|a| {
        let id = &a.ident;
        let ty = &a.ty;
        if a.is_opt || a.is_vec {
            quote! { #id: #ty, }
        } else {
            quote! { #id: std::option::Option<#ty>, }
        }
    });
    let builder_inits = type_attrs.iter().map(|a| {
        let id = &a.ident;
        if a.is_vec {
            quote! { #id: Vec::new(), }
        } else {
            quote! { #id: None, }
        }
    });
    let build_outs = type_attrs.iter().map(|a| {
        let id = &a.ident;
        if a.is_opt || a.is_vec {
            quote! { #id: self.#id.clone(), }
        } else {
            quote! { #id: self.#id.clone()?, }
        }
    });
    let each_names_buf = Rc::new(RefCell::new(HashSet::new()));
    let builder_each_setters = type_attrs.iter().map(|a| {
        let id = &a.ident;
        let ty = &a.inner_type;
        if let Some(Ok(each_name)) = &a.each {
            each_names_buf.borrow_mut().insert(each_name.clone());
            let each_name = format_ident!("{}", each_name);
            Some(quote! {
                pub fn #each_name(&mut self, #id: #ty) -> &mut Self {
                    self.#id.push(#id);
                    self
                }
            })
        } else {
            None
        }
    });
    let each_names = each_names_buf.clone();
    let builder_setters = type_attrs.iter().map(|a| {
        let id = &a.ident;
        let ids: String = format!("{}", id.as_ref().unwrap());
        if each_names.borrow().contains(&ids) {
            return None;
        }
        let ty = if a.is_opt {
            a.inner_type.clone()
        } else {
            Some(a.ty.clone())
        };
        let set = if a.is_vec {
            quote! { self.#id = #id; }
        } else {
            quote! { self.#id = std::option::Option::Some(#id); }
        };
        Some(quote! {
            pub fn #id(&mut self, #id: #ty) -> &mut Self {
                #set
                self
            }
        })
    });

    let code_ts = quote! {
        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_inits)*
                }
            }
        }
        pub struct #builder_name {
            #(#builder_declares)*
        }
        impl #builder_name {
            #(#builder_each_setters)*
            #(#builder_setters)*

            pub fn build(&self) -> std::option::Option<#name> {
                Some(#name {
                    #(#build_outs)*
                })
            }
        }
    };
    Ok(code_ts.into())
}
