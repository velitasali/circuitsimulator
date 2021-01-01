//! Backward-compatibility bridge for `cs_engine::sim_log`.
//! Redirects to the unified `cs_engine::logging` module.

pub use crate::logging::{
    clear_simulation as clear, drain_simulation as drain, init_script_logging, log_sim as push,
};
