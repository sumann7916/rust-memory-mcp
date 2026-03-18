use memory_mcp::tools::SaveMemoryParams;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(SaveMemoryParams);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
