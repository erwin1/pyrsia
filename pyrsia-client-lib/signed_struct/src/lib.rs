//! This defines the derive(SignedStruct) macro. See the documentation for the Signed trait for documentation.

extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use std::collections::HashSet;

use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, DeriveInput, Field, FieldsNamed, Type, TypeReference, Visibility, Lifetime};

/// Use this macro before a struct to make it a signed struct. That means it will have signed JSON
/// associated with it. A test that shows the use of this is in the `signed-struct-test` crate.
///
/// This macro will generate additional field(s) to support the signed JSON. The struct will
/// implement the `pyrsia_client_lib::Signed` trait that includes methods for signing the struct,
/// verifying the signature(s) (multiple signatures are allowed), getting the JSON and creating a
/// new struct from the JSON.
///
/// Signed structs should be in a module by themselves so that other code cannot directly reference
/// their private fields. The macro issues an error if it has any public fields. To access the
/// fields, you should use the getters and setters that the macro generates.
///
/// Getters are generated with the signature `fn field(&self) -> &type`.
///
/// Setters are generated as `fn field(&mut self, val: type)`. In addition to setting their field,
/// the setters also call the `clear_json()` method provided by the `Signed` trait. This removes
/// any JSON currently associated with the struct because it is no longer valid after the struct's
/// field has been modified.
#[proc_macro_attribute]
pub fn signed_struct(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = parse_macro_input!(args as AttributeArgs);
    let mut ast = parse_macro_input!(input as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => match &mut struct_data.fields {
            syn::Fields::Named(fields) => {
                match unique_json_field_ident(fields) {
                    Ok(json_field_name) => {
                        let json_field = construct_json_field(&json_field_name);
                        fields.named.push(json_field);
                    }
                    Err(error) => return error.to_compile_error().into(),
                }
                println!("generating output");
                let output = quote! {
                #[derive(serde::Serialize, serde::Deserialize)]
                #[derive(signed_struct::SignedStructDerive)]
                #ast
                }
                .into();
                println!("Output for signed_struct: {}", output);
                output
            }
            _ => syn::parse::Error::new(
                ast.span(),
                "signed_struct may only be used with structs having named fields.",
            )
            .to_compile_error()
            .into(),
        },
        _ => syn::parse::Error::new(ast.span(), "signed_struct may only be used with structs ")
            .to_compile_error()
            .into(),
    }
}

fn construct_json_field(field_name: &Ident) -> Field {
    let json_fields_named: syn::FieldsNamed = syn::parse2(
        quote!( {
            #[serde(skip)]
            #field_name : Option<String>
        } )
        .into(),
    )
    .unwrap();
    let json_field: Field = json_fields_named.named.first().unwrap().to_owned();
    json_field
}

fn unique_json_field_ident(fields: &FieldsNamed) -> Result<Ident, syn::parse::Error> {
    let mut field_names: HashSet<String> = HashSet::new();
    for field in fields.named.iter() {
        if field.vis != Visibility::Inherited {
            return Err(syn::parse::Error::new(
                field.span(),
                "signed_struct requires all fields to be private",
            ));
        }
        for id in field.ident.iter() {
            field_names.insert(id.to_string());
        }
    }
    let mut counter = 0;
    loop {
        let mut candidate_name = String::from("_json");
        candidate_name.push_str(&counter.to_string());
        if !field_names.contains(candidate_name.as_str()) {
            return Ok(format_ident!("_json{}", counter.to_string()));
        }
        counter += 1;
    }
}

#[proc_macro_derive(SignedStructDerive)]
pub fn signed_struct_derive(input: TokenStream) -> TokenStream {
    println!("parsing input");
    let ast = parse_macro_input!(input as DeriveInput);
    match &ast.data {
        syn::Data::Struct(ref struct_data) => {
            println!("data matches Struct");
            match &struct_data.fields {
                syn::Fields::Named(fields) => {
                    println!("Struct contains named fields");
                    match scan_fields(fields) {
                        Ok((json_field_name, type_vec, field_ident_vec, setter_name_vec)) => {
                            println!("generating output from signed_struct_derive");
                            let struct_ident = &ast.ident;
                            let output = quote! {
                                impl<'π> ::pyrsia_client_lib::signed::Signed<'π> for #struct_ident<'π> {
                                    fn json(&self) -> Option<String> {
                                        self.#json_field_name.to_owned()
                                    }

                                    fn clear_json(&mut self) {
                                        self.#json_field_name = None;
                                    }

                                    fn set_json(&mut self, json: &str) {
                                        self.#json_field_name = Option::Some(json.to_string())
                                    }
                                }

                                impl<'π> #struct_ident<'π> {
                                    fn new(#( #field_ident_vec : #type_vec),*) -> #struct_ident {
                                        #struct_ident{ #(#field_ident_vec),* , #json_field_name: None }
                                    }

                                    #(fn #field_ident_vec(&self)->&#type_vec{&self.#field_ident_vec}

                                      fn #setter_name_vec(&mut self, value: #type_vec){self.#field_ident_vec = value}

                                    )*
                                }
                            }
                            .into();
                            println!("Output from signed_struct_derive: {}", output);
                            return output;
                        }
                        Err(error) => error.to_compile_error().into(),
                    }
                }
                _ => {
                    return syn::parse::Error::new(
                        ast.span(),
                        "signed_struct_derive may only be used with structs having named fields.",
                    )
                    .to_compile_error()
                    .into()
                }
            }
        }
        _ => {
            return syn::parse::Error::new(
                ast.span(),
                "signed_struct_derive may only be used with structs ",
            )
            .to_compile_error()
            .into()
        }
    }
}

fn scan_fields(
    fields: &FieldsNamed,
) -> Result<
    (
        Ident, // the name of the JSON field.
        Vec<Type>, // the types of the non-generated fields
        Vec<Ident>, // the names of the fields to be used as getters
        Vec<Ident>, // the name of the setters
    ),
    syn::parse::Error,
> {
    let mut type_vec = Vec::new();
    let mut field_name_vec: Vec<Ident> = Vec::new();
    let mut setter_name_vec: Vec<Ident> = Vec::new();
    for field in fields.named.iter() {
        let param_type = field_type_with_modified_lifetime(field);
        type_vec.push(param_type.clone());
        let field_name = field.ident.clone().unwrap();
        field_name_vec.push(field_name.clone());
        setter_name_vec.push(format_ident!("set_{}", field_name));
    }
    setter_name_vec.pop();
    match type_vec.pop() {
        Some(_) => Ok((
            field_name_vec.pop().unwrap(),
            type_vec,
            field_name_vec,
            setter_name_vec,
        )),
        None => Err(syn::parse::Error::new(
            fields.span(),
            "signed_struct_derive does not work with an empty struct",
        )),
    }
}

fn field_type_with_modified_lifetime(field: &Field) -> Type {
    match field.ty {
        syn::Type::Reference(ref t) => {
            let mut ty = t.clone();
            ty.lifetime = Some(Lifetime::new("'π", Span::call_site()));
            syn::Type::Reference(ty)
        }
        _ => field.ty.clone(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
