extern crate proc_macro;
mod context;

use context::Context;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput};

fn scannable_impl(ctx: &mut Context, ast: &syn::DeriveInput) -> Option<TokenStream> {
    let data = match &ast.data {
        syn::Data::Struct(ds) => ds,
        _ => {
            ctx.error_spanned_by(ast, "#[derive(Scannable)] is only supported on structs.");
            return None;
        }
    };
    let name = &ast.ident;
    let name_str = syn::LitStr::new(&format!("{}", name), ast.ident.span());

    let mut offset_code = quote! {};
    let mut read_code = quote! {use memscanner::ScannableValue;};
    for f in data.fields.iter() {
        let ident = f.ident.as_ref().unwrap();

        // Construct new identifiers.
        let ident_str = syn::LitStr::new(&format!("{}", ident), f.ty.span());
        let offset = format_ident!("{}_offset", ident);

        // The code that looks up the field's offset in the config and saves it.
        offset_code.extend(quote! {
            let #offset = config
                .fields
                .get(#ident_str)
                .ok_or(format_err!("{} field offset not found", #ident_str))?
                .clone();
        });

        // The code that reads the field's value and stores it in the object.
        read_code.extend(quote! {
            obj.#ident.scan_val(mem, #offset + base_addr)
                .map_err(|e| format_err!("can't read {}: {}", #ident_str, e))?;
        });
    }

    // Resolver and Scanner are implemented as closures so that the we can
    // store the config and offsets without leading new types.
    let code = quote! {
        impl memscanner::Scannable for #name {

            fn get_resolver(config: memscanner::TypeConfig) -> Result<Box<memscanner::Resolver<Self>>, failure::Error> {
                #offset_code

                let resolver = move |mem: &dyn memscanner::MemReader,
                                    start_addr: u64,
                                    end_addr: u64|
                    -> Result<Box<memscanner::Scanner<Self>>, failure::Error> {
                    let base_addr = config
                        .signature
                        .resolve(mem, start_addr, end_addr)
                        .ok_or(format_err! {"Can't resolve base address"})?;

                    let scanner = move |obj: &mut Self, mem: &dyn memscanner::MemReader| -> Result<(), failure::Error> {
                        #read_code
                        Ok(())
                    };
                    Ok(Box::new(scanner))
                };
                Ok(Box::new(resolver))
            }

            fn get_array_resolver(config: memscanner::TypeConfig)
                -> Result<Box<memscanner::ArrayResolver<#name>>, failure::Error> {
                use failure::format_err;
                let array_config = config.array
                    .as_ref()
                    .ok_or(format_err!("Can't create resolver for Vec<{}>: no array config.", #name_str))?.clone();

                #offset_code

                let resolver = move |mem: &dyn memscanner::MemReader,
                                    start_addr: u64,
                                    end_addr: u64|
                    -> Result<Box<memscanner::ArrayScanner<Self>>, failure::Error> {
                    let base_addr = config
                        .signature
                        .resolve(mem, start_addr, end_addr)
                        .ok_or(format_err! {"Can't resolve base address"})?;
                    let array_config = array_config.clone();

                    let scanner = move |vec: &mut Vec<#name>, mem: &dyn memscanner::MemReader|
                        -> Result<(), failure::Error> {
                        use std::ops::IndexMut;
                        use memscanner::MemReader;
                        use memscanner::test::TestMemReader;
                        use memscanner::macro_helpers::*;

                        // This requires that the type implement Default.
                        vec.resize_with(array_config.element_count as usize,
                            Default::default);

                        let mut cached_mem = new_mem_cache(&array_config);

                        for i in 0..(array_config.element_count as usize){
                            let obj = vec.index_mut(i);
                            let base_addr = get_array_base_addr(
                                &array_config,
                                base_addr,
                                i,
                                mem)?;

                            // Pointer tables can have null entries.  Set those to the default value.
                            if base_addr == 0x0 {
                                *obj = Default::default();
                                continue;
                            }

                            update_mem_cache(mem, &mut cached_mem, base_addr, array_config.element_size)
                                .map_err(|e| format_err!("{} of {}: ", i, #name_str))?;

                            #read_code
                       }
                        Ok(())
                    };
                    Ok(Box::new(scanner))
                };
                Ok(Box::new(resolver))
            }
        }
    };

    Some(code)
}

#[proc_macro_derive(Scannable)]
pub fn scannable_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ctx = Context::new();
    // Parse the input tokens into a syntax tree
    let ast = parse_macro_input!(input as DeriveInput);

    let code = scannable_impl(&mut ctx, &ast);

    match ctx.check() {
        Ok(_) => match code {
            Some(c) => {
                //eprintln!("code: {}", c);
                proc_macro::TokenStream::from(c)
            }
            None => {
                return quote! {compile_error!("Unknown error with #[derive(Scannable)]")}.into()
            }
        },
        Err(e) => Context::convert_to_compile_errors(e).into(),
    }
}

fn scannable_enum_impl(ctx: &mut Context, ast: &syn::DeriveInput) -> Option<TokenStream> {
    match &ast.data {
        syn::Data::Enum(_) => (),
        _ => {
            ctx.error_spanned_by(ast, "#[derive(ScannableEnum)] is only supported on enums.");
            return None;
        }
    };
    let name = &ast.ident;
    let code = quote! {
        impl memscanner::ScannableValue<#name> for #name {
            fn scan_val(&mut self, mem: &dyn memscanner::MemReader, addr: u64)
            -> Result<(), Error> {
                memscanner::macro_helpers::read_enum(self, mem, addr)
            }
        }
    };
    Some(code)
}
#[proc_macro_derive(ScannableEnum)]
pub fn scannable_enum_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ctx = Context::new();
    // Parse the input tokens into a syntax tree
    let ast = parse_macro_input!(input as DeriveInput);

    let code = scannable_enum_impl(&mut ctx, &ast);

    match ctx.check() {
        Ok(_) => match code {
            Some(c) => {
                //eprintln!("code: {}", c);
                proc_macro::TokenStream::from(c)
            }
            None => {
                return quote! {compile_error!("Unknown error with #[derive(Scannable)]")}.into()
            }
        },
        Err(e) => Context::convert_to_compile_errors(e).into(),
    }
}
