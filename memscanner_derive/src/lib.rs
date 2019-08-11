extern crate proc_macro;

mod context;

use context::Context;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput, Type};

// Returns a string representing the type of `ty.  Ensures that the
// type is supported.
fn get_type(ctx: &mut Context, ty: &Type) -> Option<&'static str> {
    match ty {
        Type::Path(p) => {
            if p.path.is_ident("u8") {
                Some("u8")
            } else if p.path.is_ident("i16") {
                Some("i16")
            } else if p.path.is_ident("u16") {
                Some("u16")
            } else if p.path.is_ident("i32") {
                Some("i32")
            } else if p.path.is_ident("u32") {
                Some("u32")
            } else if p.path.is_ident("i64") {
                Some("i64")
            } else if p.path.is_ident("u64") {
                Some("u64")
            } else if p.path.is_ident("f32") {
                Some("f32")
            } else if p.path.is_ident("f64") {
                Some("f64")
            } else {
                ctx.error_spanned_by(
                    ty.clone(),
                    format!("fields of type {:?} are unsupported.", &p),
                );
                None
            }
        }
        _ => None,
    }
}

fn scannable_impl(ctx: &mut Context, ast: &syn::DeriveInput) -> Option<TokenStream> {
    let data = match &ast.data {
        syn::Data::Struct(ds) => ds,
        _ => {
            ctx.error_spanned_by(ast, "#[derive(Scannable)] is only supported on structs.");
            return None;
        }
    };
    let name = &ast.ident;

    let mut offset_code = quote! {};
    let mut addr_code = quote! {};
    let mut read_code = quote! {};
    for f in data.fields.iter() {
        let ident = f.ident.as_ref().unwrap();

        // Construct new identifiers.
        let ident_str = syn::LitStr::new(&format!("{}", ident), f.ty.span());
        let offset = format_ident!("{}_offset", ident);
        let addr = format_ident!("{}_addr", ident);

        let read = match get_type(ctx, &f.ty) {
            Some(ty) => format_ident!("read_{}", ty),
            None => continue,
        };

        // The code that looks up the field's offset in the config and saves it.
        offset_code.extend(quote! {
            let #offset = config
                .fields
                .get(#ident_str)
                .ok_or(format_err!("{} field offset not found", #ident_str))?
                .clone();
        });

        // The code that calculates the field's address from the base address and
        // field offset.
        addr_code.extend(quote! {
            let #addr = #offset + base_addr;
        });

        // The code that reads the field's value and stores it in the object.
        read_code.extend(quote! {
            obj.#ident = mem
                .#read(#addr)
                .ok_or(format_err!("can't read {}", #ident_str))?;
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
                    #addr_code

                    let scanner = move |obj: &mut Self, mem: &dyn memscanner::MemReader| -> Result<(), failure::Error> {
                        #read_code
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
                return quote! {compile_error!("Unknown error with #[derive(Scannable)] ")}.into()
            }
        },
        Err(e) => Context::convert_to_compile_errors(e).into(),
    }
}
