//! `/cuenta/taller` — desde la dirección (GET) o desde el formulario de la página (POST).

mod _formulario;
mod _navegador;

use actix_web::web;
use serde::Deserialize;

use _formulario::taller_cuenta_formulario;
use _navegador::taller_cuenta_navegador;

/// Registra `/cuenta/taller`.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/taller")
			.service(taller_cuenta_navegador)
			.service(taller_cuenta_formulario),
	);
}

#[derive(Deserialize)]
pub(crate) struct TallerDeCuenta {
	nombre: String,
	cuota_mensual_mxn: Option<i32>,
}
