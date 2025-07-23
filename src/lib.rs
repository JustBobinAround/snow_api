pub mod auth;
pub mod errors;
pub mod glider;
pub use serde;
pub mod prelude {
    pub use crate::{
        auth::{CredentialType, GlideRecordConfig},
        errors::GlideError,
        glider::{GlideRecord, GlideReference, Glideable},
    };
    pub use glide_record_proc_macro::glideable;
}
