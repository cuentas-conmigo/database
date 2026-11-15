//! `/cuenta` — la cuenta del tallerista, en JSON o en HTML.

mod _crear_navegador;
mod _entrar_navegador;
mod _ver;
pub(crate) mod alumno;
pub(crate) mod datos;
pub(crate) mod horario;
pub(crate) mod pago;
pub(crate) mod taller;

use crate::error::ApiError;
use crate::routes::comun::*;
use crate::services as db;
use crate::web::pagina;
use actix_web::{HttpResponse, http::StatusCode, web};
use sea_orm::{DatabaseConnection, DbErr};

use _crear_navegador::crear_cuenta_navegador;
use _entrar_navegador::entrar_cuenta_navegador;
use _ver::cuenta;

/// Registra `/cuenta` y sus partes.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/cuenta")
			.service(cuenta)
			.service(crear_cuenta_navegador)
			.service(entrar_cuenta_navegador)
			.configure(datos::config)
			.configure(taller::config)
			.configure(horario::config)
			.configure(alumno::config)
			.configure(pago::config),
	);
}

pub(crate) async fn cuenta_con_error(
	db: &DatabaseConnection,
	sesion: &db::Sesion,
	error: DbErr,
) -> Result<HttpResponse, ApiError> {
	let cuerpo = db::cuenta(db, sesion).await.map_err(ApiError::db)?;
	Ok(html_estado(
		StatusCode::BAD_REQUEST,
		pagina::cuenta(&cuerpo, Some(&texto_aviso(error))),
	))
}

pub(crate) async fn responder_cuenta(
	db: &DatabaseConnection,
	entrada: &db::Entrada,
	status: StatusCode,
) -> Result<HttpResponse, ApiError> {
	let cuerpo = db::cuenta(db, &entrada.sesion).await.map_err(ApiError::db)?;
	Ok(
		HttpResponse::build(status)
			.cookie(cookie_sesion(&entrada.token))
			.json(cuerpo),
	)
}
