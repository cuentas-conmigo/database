//! `/cuenta/alumno` — desde la dirección (GET) o desde el formulario de la página (POST).

mod _formulario;
mod _navegador;

use actix_web::web;
use serde::Deserialize;

use _formulario::alumno_cuenta_formulario;
use _navegador::alumno_cuenta_navegador;

/// Registra `/cuenta/alumno`.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/alumno")
			.service(alumno_cuenta_navegador)
			.service(alumno_cuenta_formulario),
	);
}

#[derive(Deserialize)]
pub(crate) struct AlumnoDeCuenta {
	nombre: String,
	taller: Option<String>,
}
