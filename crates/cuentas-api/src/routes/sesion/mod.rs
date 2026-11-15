//! `/v1/sesion` — entrar con el usuario y la contraseña del tallerista (sin sesión guardada: la app manda el login en cada petición).

mod _crear;

use actix_web::web;

use _crear::crear_sesion;

/// Registra `/sesion` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(web::scope("/sesion").service(crear_sesion));
}

/// OpenAPI del recurso.
#[derive(utoipa::OpenApi)]
#[openapi(paths(_crear::crear_sesion))]
pub struct ApiDoc;
