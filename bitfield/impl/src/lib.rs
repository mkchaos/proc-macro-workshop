use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, spanned::Spanned};
use syn::{parse_macro_input, parse_quote, DeriveInput, Result};

#[proc_macro_attribute]
pub fn bitfield(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as syn::ItemStruct);
    let item_name = &item.ident;
    use syn::{Fields, FieldsNamed};
    if let Fields::Named(FieldsNamed { named, .. }) = &item.fields {
        let mut cur_acc_bits = quote!(0);
        let (getters, setters): (Vec<_>, Vec<_>) = named
            .iter()
            .map(|f| {
                let ty = &f.ty;
                let getter = ex_getter_fn(f, &cur_acc_bits);
                let setter = ex_setter_fn(f, &cur_acc_bits);
                cur_acc_bits = quote!(#cur_acc_bits + #ty::BITS);
                (getter, setter)
            })
            .unzip();
        let size = cur_acc_bits;
        let data = quote!(pub data: [u8; (#size) / 8]);
        item.fields = Fields::Named(parse_quote!({#data}));
        let impl_ts = quote! {
            impl #item_name {
                pub fn new() -> Self {
                    let _ : checks::MultipleOfEight<[(); (#size) % 8]>;
                    Self {
                        data: [0; (#size) / 8],
                    }
                }

                #(#getters)*
                #(#setters)*
            }
        };
        let item_with_repr_c = quote! {
            #[repr(C)]
            #item
        };
        quote!(#item_with_repr_c #impl_ts).into()
    } else {
        let err_ts = syn::Error::new(item.__span(), "bitfield only works on named structs")
            .to_compile_error();
        quote!(#item #err_ts).into()
    }
}

fn ex_getter_fn(f: &syn::Field, offset_ts: &TokenStream2) -> TokenStream2 {
    let ty = &f.ty;
    let u_ty_ts = quote!(<#ty as Specifier>::U);
    let method_name = format_ident!("get_{}", f.ident.as_ref().unwrap());
    quote!(
        pub fn #method_name(&self) -> #u_ty_ts {
            let st = #offset_ts;
            #ty::get(&self.data, #offset_ts)
        }
    )
}

fn ex_setter_fn(f: &syn::Field, offset_ts: &TokenStream2) -> TokenStream2 {
    let ty = &f.ty;
    let u_ty_ts = quote!(<#ty as Specifier>::U);
    let method_name = format_ident!("set_{}", f.ident.as_ref().unwrap());
    quote!(
        pub fn #method_name(&mut self, val: #u_ty_ts) {
            let st = #offset_ts;
            #ty::set(&mut self.data, #offset_ts, val);
        }
    )
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match expand_bit_specifier(&ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_bit_specifier(ast: &DeriveInput) -> Result<TokenStream2> {
    use syn::{Data, DataEnum};
    let name = &ast.ident;
    // let mut variants_counter = 0usize;
    // let mut expr = Option::<Expr>::None;
    // let mut expr_after_counter = 0usize;
    // let mut exprs_need_check = Vec::<TokenStream2>::new();
    // #[allow(unused_assignments)]
    // let mut u64_from_items = Vec::new();
    let variants = if let Data::Enum(DataEnum { variants, .. }) = &ast.data {
        variants
    } else {
        return Err(syn::Error::new(
            ast.__span(),
            "bitfield_specifier only works on enums",
        ));
    };
    let enum_values = variants.iter().map(|v| {
        let id = &v.ident;
        quote!(
            #name::#id
        )
    });
    let enum_length = variants.iter().len();
    // let mut enum_value = 0_u64;
    // let u64s = variants.iter().clone().map(|v| {
    //     if let Some((_, e)) = &v.discriminant {
    //         if expr.is_some() {
    //             exprs_need_check.push(quote!(#expr+#expr_after_counter));
    //         }
    //         expr = Some(e.clone());
    //         expr_after_counter = 0;
    //     } else {
    //         expr_after_counter += 1;
    //     }
    //     quote!(
            
    //     )
    // });
    // for variant in variants {
    //     variants_counter += 1;
    //     if let Some((_, e)) = &variant.discriminant {
    //         if expr.is_some() {
    //             exprs_need_check.push(quote!(#expr+#expr_after_counter));
    //         }
    //         expr = Some(e.clone());
    //         expr_after_counter = 0;
    //     } else {
    //         expr_after_counter += 1;
    //     }
    // }
    // if expr.is_some() {
    //     exprs_need_check.push(quote!(#expr+#expr_after_counter));
    // }
    let res_ts = quote! {
        impl Specifier for #name {
            const BITS: usize = get_bits_from_length(#enum_length);
            type U = Self;

            fn set(data: &mut [u8], offset: usize, val: Self::U) {
                set_data(data, offset, val as u64, Self::BITS);
            }

            fn get(data: &[u8], offset: usize) -> Self::U {
                let val = get_data(data, offset, Self::BITS);
                match val {
                    #(x if x == #enum_values as u64 => #enum_values,)*
                    _ => unreachable!(),
                }
            }
        }
    };
    Ok(res_ts)
}

#[proc_macro]
pub fn impl_bits_specifiers(_input: TokenStream) -> TokenStream {
    let mut res_ts = TokenStream2::new();
    for i in 1usize..=64 {
        let name = format_ident!("B{}", i);
        let bits = i;
        let type_name = match i {
            1..=8 => format_ident!("u8"),
            9..=16 => format_ident!("u16"),
            17..=32 => format_ident!("u32"),
            33..=64 => format_ident!("u64"),
            _ => unreachable!(),
        };
        let ts = quote! {
            impl_specifier!(#name, #bits, #type_name);
        };
        res_ts.extend(ts);
    }
    res_ts.into()
}
