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

type FieldList = syn::punctuated::Punctuated<syn::Field, syn::token::Comma>;

fn join_path_segment(
    pss: &syn::punctuated::Punctuated<syn::PathSegment, syn::token::Colon2>,
) -> String {
    let list: Vec<_> = pss.iter().map(|ps| ps.ident.to_string()).collect();
    list.join("::")
}

fn get_ty_bound(id: &syn::Ident, ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        let f = segments.first()?;
        if f.ident.to_string() == id.to_string() {
            return Some(join_path_segment(segments));
        }
        let las = segments.last()?;
        if las.ident != "PhantomData" {
            if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                args,
                ..
            }) = &las.arguments
            {
                for arg in args {
                    if let syn::GenericArgument::Type(ty) = arg {
                        if let Some(s) = get_ty_bound(id, ty) {
                            return Some(s);
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_tp_trait_bound(tp: &syn::TypeParam, fields: &FieldList) -> Vec<syn::WherePredicate> {
    let mut bounds = Vec::new();
    for f in fields.iter() {
        let ty = &f.ty;
        if let Some(s) = get_ty_bound(&tp.ident, ty) {
            let ts: TS2 = s.parse().unwrap();
            bounds.push(parse_quote!(#ts: std::fmt::Debug));
        }
    }
    bounds
}

fn add_bounds_to_generics(g: &syn::Generics, fields: &FieldList) -> syn::Generics {
    let mut mg = g.clone();
    let where_clause = mg.make_where_clause();
    for p in &g.params {
        if let syn::GenericParam::Type(tp) = p {
            for b in get_tp_trait_bound(tp, fields).into_iter() {
                where_clause.predicates.push(b);
            }
        }
    }
    mg
}

fn add_bounds_to_generics_from_attr(g: &syn::Generics, attr: &syn::Attribute) -> Result<syn::Generics> {
    let mut mg = g.clone();
    let where_clause = mg.make_where_clause();
    let meta = match attr.parse_meta() {
        Ok(meta) => meta,
        Err(e) => return Err(e),
    };
    if let syn::Meta::List(list) = &meta {
        if list.nested.len() == 1 {
            let a = list.nested.first().unwrap();
            if let syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) = a {
                if nv.path.get_ident().unwrap() == "bound" {
                    if let syn::Lit::Str(ls) = &nv.lit {
                        let ts: TS2 = ls.value().parse()?;
                        where_clause.predicates.push(parse_quote!(#ts));
                        return Ok(mg);
                    }
                }
            }
        }
    }
    Err(syn::Error::new(
        meta.span(),
        r#"expected `debug(bound = "...")`"#,
    ))
}

fn expand(ast: &DeriveInput) -> Result<TS2> {
    let name = &ast.ident;
    let sname = format!("{}", name);
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = &ast.data
    {
        named
    } else {
        return Err(syn::Error::new(ast.span(), "Not a name struct"));
    };
    let g = if ast.attrs.len() > 0 {
        add_bounds_to_generics_from_attr(&ast.generics, ast.attrs.first().unwrap())?
    } else {
        add_bounds_to_generics(&ast.generics, fields)
    };
    let (impl_generics, ty_generics, where_clause) = g.split_for_impl();
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
