/// Implement [`Debug`] and skip [`None`] fields
///
/// Implement [`Debug`] for a struct which has only [`Option`] fields.
///
/// # Example
///
/// ```rust
/// struct Abc {
///     field_a: Option<u8>,
///     field_b: Option<String>,
///     field_c: Option<bool>,
/// }
/// c_dns::debug_unwrap_option_fields!(Abc, field_a, field_b, field_c,);
/// ```
#[macro_export]
macro_rules! debug_unwrap_option_fields {
    ($struct:ty, $($field:ident,)+) => {
        impl std::fmt::Debug for $struct {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut ds = f.debug_struct(stringify!($struct));
                $crate::debug_unwrap_option_single_field!(self, ds, $($field,)+);
                $crate::debug_extra_values!(self, ds, extra_values);
                ds.finish()
            }
        }
    }
}

/// Helper macro to implement [`debug_unwrap_option_fields`]
///
/// The macro can also be usefull when implementing [`Debug`] when not all fields are [`Option`].
/// In that case [`debug_unwrap_option_fields`] cannot be used.
#[macro_export]
macro_rules! debug_unwrap_option_single_field {
    ($self:ident, $ds:ident, $($field:ident,)+) => {
        $(
        if let Some($field) = &$self.$field {
            $ds.field(stringify!($field), &$field);
        }
        )+
    }
}

/// Print the `extra_values` in the [`Debug`] output
///
/// Prints the fields starting from -1 and decrementing the number.
#[macro_export]
macro_rules! debug_extra_values {
    ($self:ident, $ds:ident, $extra_values:ident) => {
        for (key, value) in $self.$extra_values.iter().rev() {
            $ds.field(&format!("{}", key), &value);
        }
    };
}
