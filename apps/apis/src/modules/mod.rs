//! One folder per core module:
//!   mod.rs         routes
//!   controller.rs  handlers only (extract -> call use case -> map response)
//!   dto/           one file per request payload
//!   response/      one file per response shape
//! DTOs/responses are separate from core types so the HTTP shape can change without touching use cases.
pub mod shipping_address;
pub mod user;
