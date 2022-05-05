use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned, spanned::Spanned};
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
        let check_bits: Vec<_> = named
            .iter()
            .filter_map(|f| {
                let attrs_len = f.attrs.len();
                if attrs_len == 1 {
                    let attr = &f.attrs[0];
                    let meta = attr.parse_meta().unwrap();
                    if let syn::Meta::NameValue(nv) = &meta {
                        let path = &nv.path;
                        let path_str = quote!(#path).to_string();
                        if path_str == "bits" {
                            let lit = &nv.lit;
                            let lit_str = quote!(#lit).to_string();
                            let lit_num = lit_str.parse::<usize>().unwrap();
                            // println!("bits {}", lit_num);
                            let ty = &f.ty;
                            return Some(quote_spanned!(lit.span()=>
                                const _: [(); #lit_num] = [(); <#ty as Specifier>::BITS];
                            ));
                        }
                    }
                }
                None
            })
            .collect();
        let size = cur_acc_bits;
        let data = quote!(pub data: [u8; (#size) / 8]);
        item.fields = Fields::Named(parse_quote!({#data}));
        let impl_ts = quote! {
            #(#check_bits)*
            const _ : checks::MultipleOfEight<[(); (#size) % 8]> = ();
            impl #item_name {
                pub fn new() -> Self {

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
    let variants = if let Data::Enum(DataEnum { variants, .. }) = &ast.data {
        variants
    } else {
        return Err(syn::Error::new(
            ast.__span(),
            "bitfield_specifier only works on enums",
        ));
    };
    let enum_length = variants.iter().len();
    let mut bits = 0_usize;
    loop {
        let bits_power = 1 << bits;
        if bits_power == enum_length {
            break;
        } else if bits_power > enum_length {
            return Err(syn::Error::new(
                ast.generics.__span(),
                "BitfieldSpecifier expected a number of variants which is a power of 2",
            ));
        } else {
            bits += 1;
        }
    }
    let mut expr = quote!(0);
    let mut after_expr_counter = 0usize;
    let check_enum_ranges: Vec<_> = variants.iter().map(|v| {
        if let Some((_, e)) = &v.discriminant {
            expr = quote!(#e);
            after_expr_counter = 0;
        } else {
            after_expr_counter += 1;
        }
        // valid if 0 else 1
        let check_array_size = quote!(
            {
                const tmp: usize = (#expr) as usize + #after_expr_counter;
                if tmp < #enum_length && tmp >= 0 { 0usize } else { 1usize }
            }
        );
        quote_spanned! ( v.ident.span()=>
            const _: checks::EnumInRange<[(); #check_array_size]> = ();
        )
    }).collect();
    let enum_values = variants.iter().map(|v| {
        let id = &v.ident;
        quote!(
            #name::#id
        )
    });
    let res_ts = quote! {
        #(#check_enum_ranges)*
        impl Specifier for #name {
            const BITS: usize = #bits;
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
