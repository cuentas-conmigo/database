//! `/cuenta/pago` — desde la dirección (GET) o desde el formulario de la página (POST).

mod _formulario;
mod _navegador;

use actix_web::web;
use serde::Deserialize;

use _formulario::pago_cuenta_formulario;
use _navegador::pago_cuenta_navegador;

/// Registra `/cuenta/pago`.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/pago")
			.service(pago_cuenta_navegador)
			.service(pago_cuenta_formulario),
	);
}

#[derive(Deserialize)]
pub(crate) struct PagoDeCuenta {
	alumno: String,
	taller: Option<String>,
	estado: Option<String>,
}
