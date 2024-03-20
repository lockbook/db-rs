use db_rs::TableId;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::*;

#[proc_macro_derive(Schema)]
pub fn schema(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let tables = match data {
        Data::Struct(DataStruct { struct_token: _, fields, semi_token: _ }) => match fields {
            Fields::Named(FieldsNamed { brace_token: _, named: tables }) => tables,
            _ => panic!("db fields must all be named"),
        },
        _ => panic!("db schema must be a struct"),
    };

    let types = tables.iter().map(|table| &table.ty);

    let idents: &Vec<&Ident> = &tables
        .iter()
        .map(|table| table.ident.as_ref().unwrap())
        .collect();

    let last = &tables
        .iter()
        .map(|table| table.ident.as_ref().unwrap())
        .last()
        .unwrap_or_else(|| panic!("No tables found!"));

    let max_tables = TableId::MAX - 1 as TableId;
    if idents.len() > max_tables as usize {
        panic!(
            "Too many tables found, the maximum is: {max_tables}. Please file an issue with db-rs."
        );
    }

    let ids: Vec<u8> = (1..(idents.len() + 1) as u8).collect();

    let output = quote! {
        use db_rs::table::Table;
        use db_rs::TableId;

        impl db_rs::Db for #ident {

            fn schema_name() -> &'static str {
                stringify!(#ident)
            }

            fn init_tables(config: db_rs::Config) -> db_rs::DbResult<Self> {
                let mut log = db_rs::Logger::init(config)?;

                #( let mut #idents = <#types>::init(#ids, log.clone()); )*

                Ok(
                    Self {
                        #( #idents, )*
                    }
                )
            }

            fn handle_event(&mut self, table_id: TableId, data: &[u8]) -> db_rs::DbResult<()> {
                match table_id {
                    #( #ids => self.#idents.handle_event(data), )*
                    _ => todo!()
                }
            }

            fn compact_log(&mut self) -> db_rs::DbResult<()> {
                use db_rs::table::Table;

                let mut data = vec![];
                #( data.append(&mut self.#idents.compact_repr()?);)*
                self.get_logger().compact_log(data)?;
                Ok(())
            }

            fn get_logger(&self) -> &db_rs::Logger {
                &self.#last.logger
            }
        }
    };
    output.into()
}
