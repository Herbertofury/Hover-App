#[cfg(windows)]
mod agent;
#[cfg(windows)]
mod branding;
pub mod cli;
#[cfg(windows)]
mod decoder_process;
#[cfg(windows)]
mod decoder_sandbox;
#[cfg(windows)]
mod platform;
#[cfg(windows)]
mod resident;
#[cfg(windows)]
mod resolver;
#[cfg(windows)]
mod zip_limits;

#[cfg(windows)]
pub fn run_decoder_worker() -> i32 {
    decoder_process::worker_entry()
}

#[cfg(windows)]
pub fn run_agent_stdio() -> i32 {
    agent::serve()
}

#[cfg(windows)]
pub fn run() -> Result<(), i32> {
    platform::run()
}

#[cfg(not(windows))]
pub fn run() -> Result<(), i32> {
    Err(1)
}
