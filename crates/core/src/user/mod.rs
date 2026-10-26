//! Public surface of the user module ("exports"). Everything else stays private.
mod application;
mod domain;

pub use application::{GetUser, Login, LoginOutput, RegisterInput, RegisterUser, UpdateProfile};
pub use domain::{Email, User, UserId, UserRepository};
