//! `/cuenta/datos` — desde la dirección (GET) o desde el formulario de la página (POST).

mod _formulario;
mod _navegador;

use actix_web::web;
use serde::Deserialize;

use _formulario::datos_cuenta_formulario;
use _navegador::datos_cuenta_navegador;

/// Registra `/cuenta/datos`.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/datos")
			.service(datos_cuenta_navegador)
			.service(datos_cuenta_formulario),
	);
}

#[derive(Deserialize)]
pub(crate) struct DatosCuenta {
	nombre: String,
	oficio: String,
}
