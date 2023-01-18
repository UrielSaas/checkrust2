extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{AngleBracketedGenericArguments, braced, GenericArgument, Ident, LitInt, parse2, Token, TypePath};
use syn::parse::{self, Parse, ParseStream};
use syn::punctuated::Punctuated;

impl Parse for Peripheral {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let register_crate = input.parse()?;
        let name = input.parse()?;
        let registers;
        braced!(registers in input);
        Ok(Self {
            register_crate,
            name,
            registers: Punctuated::parse_terminated(&registers)?,
        })
    }
}

impl Parse for Register {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let offset = input.parse()?;
        let _: Token![=>] = input.parse()?;
        let name = input.parse()?;
        let _: Token![:] = input.parse()?;
        let reg_type = input.parse()?;
        let operations;
        braced!(operations in input);
        Ok(Self {
            offset,
            name,
            reg_type,
            operations: Punctuated::parse_terminated(&operations)?,
        })
    }
}

impl Parse for Operation {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        Ok(Self {
            op_trait: input.parse()?,
            args: match input.peek(Token![<]) {
                false => Punctuated::new(),
                true => {
                    let args: AngleBracketedGenericArguments = input.parse()?;
                    args.args
                },
            },
        })
    }
}

struct Peripheral {
    register_crate: Ident,
    name: Ident,
    registers: Punctuated<Register, Token![,]>,
}

struct Register {
    offset: LitInt,
    name: Ident,
    reg_type: TypePath,
    operations: Punctuated<Operation, Token![+]>,
}

struct Operation {
    op_trait: TypePath,
    args: Punctuated<GenericArgument, Token![,]>,
}

fn peripheral_impl(input: TokenStream) -> TokenStream {
    let peripheral: Peripheral = match parse2(input) {
        Err(error) => return error.into_compile_error(),
        Ok(peripheral) => peripheral,
    };

    let name = peripheral.name;
    let fields = peripheral.registers.iter().map(|register| {
        let name = &register.name;
        let register_crate = &peripheral.register_crate;
        let offset = &register.offset;
        quote! { #name: #register_crate::Register<#offset, Self, Accessor> }
    });
    let op_impls = peripheral.registers.iter().map(|register| {
        let offset = &register.offset;
        let reg_type = &register.reg_type;
        let op_impls = register.operations.iter().map(|op| {
            let op_trait = &op.op_trait;
            let args = &op.args;
            quote! {
                impl<Accessor: #op_trait::Access<#offset, #args>> #op_trait::Has<#offset, #args> for #name<Accessor> {}
            }
        });
        quote! {
            impl<Accessor: ValueAt<offset, Value = #reg_type>> ValueAt<offset> for #name<Accessor> {
				
            }
            #(#op_impls)* 
			
    });
    let struct_definition = quote! {
        struct #name<Accessor> {
            #(#fields),*
        }
        #(#op_impls)*
    };
    quote! {
        #struct_definition
    }
}

#[proc_macro]
pub fn peripheral(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    peripheral_impl(input.into()).into()
}
