//! `/v1/webhooks` — avisos de Pey cuando llega un pago. Públicos; se verifica su firma.

mod _pey;

use actix_web::web;

use _pey::webhook_pey;

/// Registra `/webhooks` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(web::scope("/webhooks").service(webhook_pey));
}

/// OpenAPI del recurso.
#[derive(utoipa::OpenApi)]
#[openapi(paths(_pey::webhook_pey))]
pub struct ApiDoc;
