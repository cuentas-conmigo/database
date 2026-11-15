//! `/reportes` — Reportes: resumen, cuotas e ingresos.

mod _cuotas;
mod _ingresos;
mod _resumen;

use actix_web::web;

use _cuotas::listar_cuotas;
use _ingresos::listar_ingresos;
use _resumen::resumen;

/// Registra los recursos de reportes, cada uno con su ruta.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg
		.service(web::scope("/resumen").service(resumen))
		.service(web::scope("/cuotas").service(listar_cuotas))
		.service(web::scope("/ingresos").service(listar_ingresos));
}
