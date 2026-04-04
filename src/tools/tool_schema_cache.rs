use std::sync::OnceLock;

#[allow(dead_code)]
pub static TOOL_SCHEMA_CACHE: OnceLock<String> = OnceLock::new();

#[allow(dead_code)]
pub fn get_or_init_cache() -> &'static str {
    TOOL_SCHEMA_CACHE.get_or_init(|| {
        r#"[
            {
                "name": "SlimLspTool",
                "description": "Reads cargo check output explicitly mapping diagnostics, compiler suggestions, and broken lines natively without background socket locking.",
                "parameters": {}
            },
            {
                "name": "FileReadTool",
                "description": "Reads raw file text natively tracking the memory boundaries securely."
            },
            {
                "name": "GuardTool",
                "description": "Resolves exact canonical pathing shielding internal directory structures dynamically."
            }
        ]"#.to_string()
    })
}
