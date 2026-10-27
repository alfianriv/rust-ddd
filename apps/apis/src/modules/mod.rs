//! One file per core module: request/response DTOs + handlers + routes.
//! DTOs are separate from core types so the HTTP shape can change without touching use cases.
pub mod shipping_address;
pub mod user;
