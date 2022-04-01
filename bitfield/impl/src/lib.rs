use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, spanned::Spanned};
use syn::{parse_macro_input, parse_quote};

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let mut item = parse_macro_input!(input as syn::ItemStruct);
    let item_name = &item.ident;
    use syn::{Fields, FieldsNamed};
    if let Fields::Named(FieldsNamed { named, .. }) = &item.fields {
        let mut cur_acc_bits = quote!(0);
        let (getters, setters): (Vec<_>, Vec<_>) = named
            .iter()
            .map(|f| {
                let ty = &f.ty;
                let getter = expand_getter_fn(f, &cur_acc_bits);
                let setter = expand_setter_fn(f, &cur_acc_bits);
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
                    Self {
                        data: [0; (#size) / 8],
                    }
                }

                #(#getters)*
                #(#setters)*
            }
        };
        quote!(#item #impl_ts).into()
    } else {
        let err_ts = syn::Error::new(item.__span(), "bitfield only works on named structs")
            .to_compile_error();
        quote!(#item #err_ts).into()
    }
}

fn expand_getter_fn(f: &syn::Field, offset_ts: &TokenStream2) -> TokenStream2 {
    let ty = &f.ty;
    let en_offset_ts = quote!(#offset_ts + #ty::BITS);
    let u_ty_ts = quote!(<#ty as Specifier>::U);
    let method_name = format_ident!("get_{}", f.ident.as_ref().unwrap());
    quote!(
        pub fn #method_name(&self) -> #u_ty_ts {
            let st = #offset_ts;
            let en = #en_offset_ts - 1;
            let st_u8 = st / 8;
            let en_u8 = en / 8;
            let mut sum_val: #u_ty_ts = 0;
            for i in st_u8..=en_u8 {
                let val = self.data[i];
                let act_st = if st < i * 8 { i * 8 } else { st };
                let act_ed = if en > (i+1) * 8 { (i+1) * 8 - 1 } else { en };
                let len = act_ed - act_st + 1;
                let st_offset = act_st - i * 8;
                let mask = if len < 8 { ((1 << len) - 1) << st_offset } else { 0xff };
                let act_val = ((val & mask) >> st_offset) as #u_ty_ts;  
                sum_val |= (act_val << (act_st - st));
            }
            sum_val
        }
    )
}

fn expand_setter_fn(f: &syn::Field, offset_ts: &TokenStream2) -> TokenStream2 {
    let ty = &f.ty;
    let en_offset_ts = quote!(#offset_ts + #ty::BITS);
    let u_ty_ts = quote!(<#ty as Specifier>::U);
    let method_name = format_ident!("set_{}", f.ident.as_ref().unwrap());
    quote!(
        pub fn #method_name(&mut self, val: #u_ty_ts) {
            let st = #offset_ts;
            let en = #en_offset_ts - 1;
            let st_u8 = st / 8;
            let en_u8 = en / 8;
            let mut sum_val: #u_ty_ts = 0;
            for i in st_u8..=en_u8 {
                let act_st = if st < i * 8 { i * 8 } else { st };
                let act_ed = if en > (i+1) * 8 { (i+1) * 8 - 1 } else { en };
                let len = act_ed - act_st + 1;
                let st_offset = act_st - i * 8;
                let mask = if len < 8 { ((1 << len) - 1) << st_offset } else { 0xff };
                let act_val = (val >> (act_st - st)) as u8;
                let data_val = self.data[i];
                let modified_val = (data_val & !mask) + (act_val << st_offset);
                self.data[i] = modified_val;
            }
        }
    )
}
