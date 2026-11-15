//! Writes the OpenAPI document of the app's `/v1` routes to `crates/cuentas-api/openapi.json`.
//!
//! `cargo run --bin openapi` (add `-- --quiet` to print nothing). CI runs it and fails when the committed file is
//! out of date.

use std::path::Path;

fn main() -> std::io::Result<()> {
	let quiet = std::env::args().any(|arg| arg == "--quiet");
	let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("openapi.json");
	let mut text = serde_json::to_string_pretty(&cuentas_api::docs::document()).map_err(std::io::Error::other)?;
	text.push('\n');
	std::fs::write(&path, text)?;
	if !quiet {
		println!("Wrote {}", path.display());
	}
	Ok(())
}
