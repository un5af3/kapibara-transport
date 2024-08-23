//! Dns

pub mod option;
pub use option::ResolveOption;

pub mod error;
pub use error::ResolveError;

pub mod resolver;
pub use resolver::Resolver;
