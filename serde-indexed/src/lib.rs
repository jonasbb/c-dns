/*! Derivation of [`Serialize`][serialize] and [`Deserialize`][deserialize] that replaces struct keys with numerical indices.

### Usage example
The macros currently understand `serde`'s [`skip_serializing_if`][skip-serializing-if] field attribute
and a custom `offset` container attribute.

```ignore
use serde_indexed::{DeserializeIndexed, SerializeIndexed};

#[derive(Clone, Debug, PartialEq, SerializeIndexed, DeserializeIndexed)]
#[serde_indexed(offset = 1)]
pub struct SomeKeys {
    pub number: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub option: Option<u8>,
    pub bytes: [u8; 7],
}
```

### Generated code example
`cargo expand --test basics` exercises the macros using [`serde_cbor`][serde-cbor].

[serialize]: https://docs.serde.rs/serde/ser/trait.Serialize.html
[deserialize]: https://docs.serde.rs/serde/de/trait.Deserialize.html
[skip-serializing-if]: https://serde.rs/field-attrs.html#skip_serializing_if
[serde-cbor]: https://docs.rs/serde_cbor
*/

mod parse;

use crate::parse::{Field, Input};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Error};

fn serialize_fields(fields: &[parse::Field], offset: isize) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let index = field.index as isize + offset;
            let ident = &field.ident;
            if let Some(path) = &field.skip_serializing_if {
                quote! {
                    if !#path(&self.#ident) {
                        map.serialize_entry(&#index, &self.#ident)?;
                    }
                }
            } else if field.collect_extras {
                quote! {
                    for (key, value) in &self.#ident {
                        map.serialize_entry(key, value)?;
                    }
                }
            } else {
                quote! {
                    map.serialize_entry(&#index, &self.#ident)?;
                }
            }
        })
        .collect()
}

fn count_serialized_fields(fields: &[parse::Field]) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let ident = &field.ident;
            if let Some(path) = &field.skip_serializing_if {
                quote! { if #path(&self.#ident) { 0 } else { 1 } }
            } else if field.collect_extras {
                quote! { self.#ident.len() }
            } else {
                quote! { 1 }
            }
        })
        .collect()
}

#[proc_macro_derive(SerializeIndexed, attributes(serde, serde_indexed))]
pub fn derive_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Input);
    let ident = input.ident;
    let num_fields = count_serialized_fields(&input.fields);
    let serialize_fields = serialize_fields(&input.fields, input.attrs.offset);
    let length = if input.attrs.emit_length {
        quote!(::std::option::Option::Some(0 #( + #num_fields)*))
    } else {
        quote!(::std::option::Option::None)
    };

    TokenStream::from(quote! {
        #[automatically_derived]
        impl serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer
            {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(#length)?;

                #(#serialize_fields)*

                map.end()
            }
        }
    })
}

fn none_fields(fields: &[parse::Field]) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let ident = format_ident!("{}", &field.label);
            quote! {
                let mut #ident = ::std::option::Option::None;
            }
        })
        .collect()
}

fn unwrap_expected_fields(fields: &[parse::Field]) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let label = field.label.clone();
            let ident = format_ident!("{}", &field.label);
            quote! {
                let #ident = match #ident {
                        ::std::option::Option::Some(#ident) => #ident,
                        ::std::option::Option::None =>
                        match crate::derive_helpers::missing_field(#label)
                            {
                            ::std::result::Result::Ok(__val) => __val,
                            ::std::result::Result::Err(__err) => {
                                return ::std::result::Result::Err(__err);
                            }
                        },
                    };
            }
        })
        .collect()
}

fn match_fields(fields: &[parse::Field], offset: isize) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let label = field.label.clone();
            let ident = format_ident!("{}", &field.label);
            let index = field.index as isize + offset;
            quote! {
                #index => {
                    if ::std::option::Option::is_some(& #ident) {
                        return ::std::result::Result::Err(serde::de::Error::duplicate_field(#label));
                    }
                    #ident = ::std::option::Option::Some(map.next_value()?);
                },
            }
        })
        .collect()
}

fn all_fields(fields: &[parse::Field]) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let ident = format_ident!("{}", &field.label);
            quote! {
                #ident
            }
        })
        .collect()
}

#[proc_macro_derive(DeserializeIndexed, attributes(serde, serde_indexed))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Input);
    let ident = input.ident;
    let mut none_fields = none_fields(&input.fields);
    let mut unwrap_expected_fields = unwrap_expected_fields(&input.fields);
    let mut match_fields = match_fields(&input.fields, input.attrs.offset);
    let all_fields = all_fields(&input.fields);

    // Check if an extras field exists, duplication is error
    // If found remove it from the initialization and unwrapping lists
    // Generate special initialization code
    // Generate code to handle negative values
    let extra_fields: Vec<&Field> = input
        .fields
        .iter()
        .filter(|field| field.collect_extras)
        .collect();
    if extra_fields.len() > 1 {
        return Error::new(
            extra_fields[1].ident.span(),
            "At most one field can be annotated with #[serde_indexed(extras)]",
        )
        .into_compile_error()
        .into();
    }
    let extra_field = extra_fields.get(0);
    let handle_extra_fields = if let Some(extra_field) = extra_field {
        none_fields.remove(extra_field.index);
        unwrap_expected_fields.remove(extra_field.index);
        match_fields.remove(extra_field.index);

        let ident = &extra_field.ident;
        let ty = &extra_field.ty;
        none_fields.push(quote! {
            let mut #ident: #ty = ::std::default::Default::default();
        });

        // Add negative fields to the extras map
        quote! {
            x if x < 0 => {
                #ident.insert(x, map.next_value()?);
            }
        }
    } else {
        // Consume negative fields and throw them away
        quote! {
            x if x < 0 => {
                let _: ::serde::de::IgnoredAny = map.next_value()?;
            }
        }
    };

    let the_loop = if !input.fields.is_empty() {
        // NB: In the previous "none_fields", we use the actual struct's
        // keys as variable names. If the struct happens to have a key
        // named "key", it would clash with __serde_indexed_internal_key,
        // if that were named key.
        quote! {
            while let Some(__serde_indexed_internal_key) = map.next_key()? {
                match __serde_indexed_internal_key {
                    #(#match_fields)*
                    #handle_extra_fields
                    _ => {
                        return Err(serde::de::Error::duplicate_field("inexistent field index"));
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    TokenStream::from(quote! {
        #[automatically_derived]
        impl<'de> serde::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct IndexedVisitor;

                impl<'de> serde::de::Visitor<'de> for IndexedVisitor {
                    type Value = #ident;

                    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                        formatter.write_str(stringify!(#ident))
                    }

                    fn visit_map<V>(self, mut map: V) -> core::result::Result<#ident, V::Error>
                    where
                        V: serde::de::MapAccess<'de>,
                    {
                        #(#none_fields)*

                        #the_loop

                        #(#unwrap_expected_fields)*

                        Ok(#ident { #(#all_fields),* })
                    }
                }

                deserializer.deserialize_map(IndexedVisitor {})
            }
        }
    })
}
