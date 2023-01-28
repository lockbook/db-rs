use db_rs::TableId;
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

    let idents1 = tables.iter().map(|table| table.ident.clone().unwrap());
    let idents2 = tables.iter().map(|table| table.ident.clone().unwrap());
    let idents3 = tables.iter().map(|table| table.ident.clone().unwrap());
    let idents4 = tables
        .iter()
        .map(|table| table.ident.clone().unwrap())
        .last()
        .unwrap();
    let types = tables.iter().map(|table| table.ty.clone());
    let ids: Vec<u8> = vec![0, 1, 2];
    // let ids: Vec<LitInt> = (0..idents.len())
    //     .into_iter()
    //     .map(|i| format!("{}_u8", i))
    //     .map(|u| LitInt::new(&u, ident.span()))
    //     .collect();

    let output = quote! {
        use db_rs::logger::Logger;
        use db_rs::Db;
        use db_rs::table::Table;

        impl Db for #ident {
            fn init(config: Config) -> Self {
                let log = Logger::init(config);
                let log_entries = log.get_entries();

                #( let mut #idents1 = <#types>::init(#ids, log.clone()); )*

                for entry in log_entries {
                    match entry.table_id {
                        #( #ids => #idents2.handle_event(entry.bytes), )*
                        _ => todo!()
                    }
                }

                Self {
                    #( #idents3, )*
                }
            }

            fn get_logger(&mut self) -> &mut Logger {
                &mut self.#idents4.logger
            }
        }
    };
    output.into()
}
