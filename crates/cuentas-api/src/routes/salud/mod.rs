//! `/v1/salud` — si la API está arriba y en qué modo habla con Pey. Pública.

mod _ver;

use actix_web::web;

use _ver::salud;

/// Registra `/salud` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(web::scope("/salud").service(salud));
}

/// OpenAPI del recurso.
#[derive(utoipa::OpenApi)]
#[openapi(paths(_ver::salud))]
pub struct ApiDoc;
