use std::io::Result;

fn main() -> Result<()> {
    // Tell Cargo to recompile if any of these files change
    println!("cargo:rerun-if-changed=proto/state.proto");
    
    // Compile the protobuf files
    prost_build::compile_protos(&["proto/state.proto"], &["proto/"])?;
    
    Ok(())
}
