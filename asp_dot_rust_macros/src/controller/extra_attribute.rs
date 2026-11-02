//! The built-in attribute macros: `#[name(value)]` is `#[attr(name = value)]` with the value typed as the declared type
//! (see `attribute.rs` for how they are expanded and read by `#[controller_route]`).
//!
//! To add one, add an entry to `for_each_attr_alias!` below: `name: type`, with the doc comment of the macro. Nothing else has to change:
//! the macro itself is defined in `lib.rs` (a `#[proc_macro_attribute]` has to be at the root of the crate) from this same list,
//! and `ATTR_ALIASES` is the table `#[controller_route]` reads.

use super::attribute::AttrAlias;

/// Calls `$callback!` with the whole list of built-in attribute macros.
macro_rules! for_each_attr_alias {
    ($callback:ident) => {
        $callback! {
            /// sets the maximum size of the request body of a route, in bytes: it is the attribute `#[attr(max_body_size = <bytes>)]`.
            /// A route without it uses `max_request_body_size` of `HyperConfig`. <br>
            /// Like `#[attr]`, on an action it applies to all its routes, and on the controller impl (below `#[controller_route]`) it is the default
            /// of all the routes of the controller, an action can override it. The size is a `usize` (a negative value or another type is a compile error).
            /// Example usage:
            ///
            /// ```no_run
            /// #[post("/upload")]
            /// #[max_body_size(10 * 1024 * 1024)]
            /// ```
            max_body_size: usize,
        }
    };
}
pub(crate) use for_each_attr_alias;

macro_rules! define_attr_alias_table {
    ($( $(#[$meta:meta])* $name:ident : $ty:ty ),* $(,)?) => {
        /// Every built-in attribute macro.
        pub(crate) const ATTR_ALIASES: &[AttrAlias] = &[
            $( AttrAlias { name: stringify!($name), ty: stringify!($ty) } ),*
        ];
    };
}
for_each_attr_alias!(define_attr_alias_table);
