//! `/v1/cuenta` — the instructor's own account from the app: sign up and delete it.

mod _borrar;
mod _crear;

use actix_web::web;

use _borrar::borrar_cuenta;
use _crear::crear_cuenta;

/// Registra `/cuenta` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(web::scope("/cuenta").service(crear_cuenta).service(borrar_cuenta));
}

/// OpenAPI del recurso.
#[derive(utoipa::OpenApi)]
#[openapi(paths(_crear::crear_cuenta, _borrar::borrar_cuenta))]
pub struct ApiDoc;
