mod iterators;
pub mod serialization;
mod utils;

/// DNS transport protocol
pub enum Transport {
    /// UDP specified in RFC 1035
    Udp = 0,
    /// TCP specified in RFC 1035
    Tcp = 1,
    /// TLS specified in RFC 7858
    Tls = 2,
    /// DTLS specified in RFC 8094
    Dtls = 3,
    /// HTTPS specified in RFC 8484
    Https = 4,
    /// Reserved Value
    Reserved = 5,
    NonStandard = 15,
}

/// Serialization helpers
///
/// These functions are necessary for the derive to produce the correct code.
#[doc(hidden)]
mod derive_helpers {
    use serde::{Deserialize, Deserializer};
    use serde::de::{Error, Visitor};
    use std::marker::PhantomData;

    /// If the missing field is of type `Option<T>` then treat is as `None`,
    /// otherwise it is an error.
    ///
    /// Original found here: https://github.com/serde-rs/serde/blob/bc7b2b1deef5755e1ef8b5c2926c0b27bdbf9753/serde/src/private/de.rs#L18-L56
    /// Original Author: David Tolnay (@dtolnay)
    pub fn missing_field<'de, V, E>(field: &'static str) -> Result<V, E>
    where
        V: Deserialize<'de>,
        E: Error,
    {
        struct MissingFieldDeserializer<E>(&'static str, PhantomData<E>);

        impl<'de, E> Deserializer<'de> for MissingFieldDeserializer<E>
        where
            E: Error,
        {
            type Error = E;

            fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, E>
            where
                V: Visitor<'de>,
            {
                Err(Error::missing_field(self.0))
            }

            fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, E>
            where
                V: Visitor<'de>,
            {
                visitor.visit_none()
            }

            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
                bytes byte_buf unit unit_struct newtype_struct seq tuple
                tuple_struct map struct enum identifier ignored_any
            }
        }

        let deserializer = MissingFieldDeserializer(field, PhantomData);
        Deserialize::deserialize(deserializer)
    }
}
