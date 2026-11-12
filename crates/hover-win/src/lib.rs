#[cfg(windows)]
mod platform;
#[cfg(windows)]
mod resolver;
#[cfg(windows)]
mod zip_limits;

#[cfg(windows)]
pub fn run() -> Result<(), i32> {
    platform::run()
}

#[cfg(not(windows))]
pub fn run() -> Result<(), i32> {
    Err(1)
}
