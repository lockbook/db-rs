use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::*;

#[proc_macro_derive(Db)]
pub fn db(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let tables = match data {
        Data::Struct(DataStruct {
            struct_token: _,
            fields,
            semi_token: _,
        }) => match fields {
            Fields::Named(FieldsNamed {
                brace_token: _,
                named: tables,
            }) => tables,
            _ => panic!("db fields must all be named"),
        },
        _ => panic!("db schema must be a struct"),
    };

    let idents: &Vec<&Ident> = &tables
        .iter()
        .map(|table| table.ident.as_ref().unwrap())
        .collect();

    let last = &tables
        .iter()
        .map(|table| table.ident.as_ref().unwrap())
        .last()
        .unwrap();

    let types = tables.iter().map(|table| &table.ty);
    let ids: Vec<u8> = (0..idents.len() as u8).into_iter().collect();

    let output = quote! {
        use db_rs::logger::Logger;
        use db_rs::Db;
        use db_rs::table::Table;
        use db_rs::DbResult;

        impl Db for #ident {
            fn init(mut config: Config) -> DbResult<Self> {
                let schema_name = stringify!(#ident);
                config.schema_name = Some(schema_name.to_string());
                let log = Logger::init(config)?;
                let log_entries = log.get_entries(log.get_bytes()?);

                #( let mut #idents = <#types>::init(#ids, log.clone()); )*

                for entry in log_entries {
                    match entry.table_id {
                        #( #ids => #idents.handle_event(entry.bytes), )*
                        _ => todo!()
                    }
                }

                Ok(
                    Self {
                        #( #idents, )*
                    }
                )
            }

            fn get_logger(&mut self) -> &mut Logger {
                &mut self.#last.logger
            }
        }
    };
    output.into()
}
