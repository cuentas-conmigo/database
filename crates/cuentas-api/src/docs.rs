//! OpenAPI document of the app's routes under `/v1`, generated at compile time from the handlers'
//! `#[utoipa::path]`. `cargo run --bin openapi` writes it to `crates/cuentas-api/openapi.json`; the API
//! does not serve it.

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::routes::docs as api;

/// Adds the tallerista login (HTTP Basic) as the `basic` security scheme.
struct BasicAuth;

impl Modify for BasicAuth {
	fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
		let components = openapi.components.get_or_insert_with(Default::default);
		components.add_security_scheme(
			"basic",
			SecurityScheme::Http(
				HttpBuilder::new()
					.scheme(HttpAuthScheme::Basic)
					.description(Some("Tallerista usuario and clave (Amanda's accounts)"))
					.build(),
			),
		);
	}
}

/// The app's contract under `/v1`.
#[derive(OpenApi)]
#[openapi(
	info(
		title = "Cuentas API",
		version = env!("CARGO_PKG_VERSION"),
		description = "Contract of the Paga con tu foto app, on top of Amanda's Cuentas Conmigo backend."
	),
	servers((url = "/")),
	modifiers(&BasicAuth),
	nest(
		(path = "/v1/salud", api = api::Salud),
		(path = "/v1/sesion", api = api::Sesion),
		(path = "/v1/cuenta", api = api::Cuenta),
		(path = "/v1/talleres", api = api::Talleres),
		(path = "/v1/alumnos", api = api::Alumnos),
		(path = "/v1/pagos", api = api::Pagos),
		(path = "/v1/webhooks", api = api::Webhooks),
	)
)]
pub struct ApiDoc;

/// The document as JSON.
pub fn document() -> serde_json::Value {
	serde_json::to_value(ApiDoc::openapi()).expect("the OpenAPI document serializes")
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeSet;

	use serde_json::Value;

	use super::document;

	fn refs(value: &Value, found: &mut BTreeSet<String>) {
		match value {
			Value::Object(map) => {
				for (key, item) in map {
					if key == "$ref" {
						if let Some(text) = item.as_str() {
							found.insert(text.to_string());
						}
					} else {
						refs(item, found);
					}
				}
			}
			Value::Array(items) => items.iter().for_each(|item| refs(item, found)),
			_ => {}
		}
	}

	fn schemas(doc: &Value) -> BTreeSet<String> {
		doc["components"]["schemas"]
			.as_object()
			.map(|map| map.keys().map(|name| format!("#/components/schemas/{name}")).collect())
			.unwrap_or_default()
	}

	#[test]
	fn every_ref_points_to_a_schema() {
		let doc = document();
		let mut found = BTreeSet::new();
		refs(&doc, &mut found);
		let missing: Vec<_> = found.difference(&schemas(&doc)).cloned().collect();
		assert!(missing.is_empty(), "dangling $ref: {missing:?}");
	}

	#[test]
	fn every_schema_is_used() {
		let doc = document();
		let mut found = BTreeSet::new();
		refs(&doc, &mut found);
		let orphans: Vec<_> = schemas(&doc).difference(&found).cloned().collect();
		assert!(orphans.is_empty(), "orphan schemas: {orphans:?}");
	}

	#[test]
	fn protected_paths_ask_for_the_login() {
		let doc = document();
		let public = ["/v1/salud", "/v1/sesion", "/v1/webhooks/pey"];
		for (path, item) in doc["paths"].as_object().expect("paths") {
			for (method, operation) in item.as_object().expect("operations") {
				let secured = operation.get("security").is_some();
				let open = public.contains(&path.as_str()) || (path == "/v1/cuenta" && method == "post");
				assert_eq!(secured, !open, "{method} {path}");
			}
		}
	}
}
