use std::io::Result;

fn main() -> Result<()> {  
    // Compile the protobuf files
    prost_build::compile_protos(&["proto/state.proto"], &["proto/"])?;
    
    Ok(())
}
