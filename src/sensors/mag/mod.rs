#[cfg(feature = "real-sensors")]
mod registry;

#[cfg(feature = "real-sensors")]
mod hmc8553l;

pub(crate) mod reader;