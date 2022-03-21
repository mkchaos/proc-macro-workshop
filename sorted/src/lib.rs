use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::visit_mut::{self, VisitMut};
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

#[proc_macro_attribute]
pub fn check(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as syn::Item);
    if let syn::Item::Fn(item_fn) = &mut item {
        let mut visitor = FnVisitor::new();
        visitor.visit_item_fn_mut(item_fn);
        if visitor.has_err() {
            let mut ts = visitor.err.unwrap().to_compile_error();
            ts.extend(quote!(#item_fn));
            return ts.into();
        }
    } else {
        unimplemented!();
    }
    quote! {
        #item
    }
    .into()
}

struct FnVisitor {
    err: Option<syn::Error>,
}

impl FnVisitor {
    fn new() -> FnVisitor {
        FnVisitor { err: None }
    }

    fn has_err(&self) -> bool {
        self.err.is_some()
    }
}

fn get_path_str(path: &syn::Path) -> String {
    let path_vec: Vec<_> = path
        .segments
        .iter()
        .map(|ps| ps.ident.to_string())
        .collect();
    path_vec.join("::")
}

impl VisitMut for FnVisitor {
    fn visit_expr_match_mut(&mut self, node: &mut syn::ExprMatch) {
        if node.attrs.len() == 1 {
            let attr = node.attrs.first().unwrap();
            if !self.has_err() && attr.path.get_ident().unwrap() == "sorted" {
                let mut order: usize = 0;
                let mut str_vec: Vec<String> = Vec::new();
                for arm in node.arms.iter() {
                    order += 1;
                    let (path_str, to_tokens): (String, Box<dyn ToTokens>) = match &arm.pat {
                        syn::Pat::Ident(id) => (id.ident.to_string(), Box::new(id.ident.clone())),
                        syn::Pat::Path(i) => (get_path_str(&i.path), Box::new(i.path.clone())),
                        syn::Pat::TupleStruct(i) => (get_path_str(&i.path), Box::new(i.path.clone())),
                        syn::Pat::Struct(i) => (get_path_str(&i.path), Box::new(i.path.clone())),
                        syn::Pat::Wild(_) => {
                            if order != node.arms.len() {
                                // error
                                unimplemented!();
                            }
                            break;
                        }
                        _pat => {
                            let emsg = "unsupported by #[sorted]";
                            let e = syn::Error::new_spanned(_pat, emsg);
                            self.err = Some(e);
                            break;
                        }
                    };
                    for s in str_vec.iter() {
                        if s.gt(&path_str) {
                            let emsg = format!("{} should sort before {}", path_str, s);
                            let e = syn::Error::new_spanned(to_tokens, emsg);
                            self.err = Some(e);
                            break;
                        }
                    }
                    str_vec.push(path_str);
                    if self.has_err() {
                        break;
                    }
                }
            }
        }
        node.attrs.clear();

        visit_mut::visit_expr_match_mut(self, node);
    }
}
