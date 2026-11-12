//! Explicit developer inspection; never executes or writes package contents.
use hover_core::CancellationToken;
use hover_formats::dbpf::{Limits, read_index};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: dbpf_index <package>")?;
    let mut file = std::fs::File::open(path)?;
    let index = read_index(&mut file, Limits::default(), &CancellationToken::new())?;
    println!(
        "declared={},inspected={},partial={}",
        index.declared_count,
        index.entries.len(),
        index.truncated
    );
    for entry in index.entries {
        println!(
            "{:08x}:{:08x}:{:016x},{}",
            entry.type_id, entry.group_id, entry.instance_id, entry.decompressed_size
        );
    }
    Ok(())
}
