//! `/cuenta/horario` — desde la dirección (GET) o desde el formulario de la página (POST).

mod _formulario;
mod _navegador;

use actix_web::web;
use serde::Deserialize;

use _formulario::horario_cuenta_formulario;
use _navegador::horario_cuenta_navegador;

/// Registra `/cuenta/horario`.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/horario")
			.service(horario_cuenta_navegador)
			.service(horario_cuenta_formulario),
	);
}

#[derive(Deserialize)]
pub(crate) struct HorarioDeCuenta {
	taller: Option<String>,
	dia: String,
	de: String,
	a: String,
}
