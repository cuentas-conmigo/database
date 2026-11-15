//! `/anuales` — Inscripciones anuales (administración).

mod _agregar_navegador;
mod _borrar;
mod _borrar_navegador;
mod _crear;
mod _listar;

use crate::error::ApiError;
use crate::models::entities::anual;
use crate::routes::comun::*;
use crate::services as db;
use actix_web::{HttpResponse, web};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde::Deserialize;

use _agregar_navegador::anual_agregar_navegador;
use _borrar::borrar_anual;
use _borrar_navegador::anual_borrar_navegador;
use _crear::crear_anual;
use _listar::listar_anuales;

/// Registra `/anuales` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/anuales")
			.service(listar_anuales)
			.service(crear_anual)
			.service(anual_agregar_navegador)
			.service(anual_borrar_navegador)
			.service(borrar_anual),
	);
}

#[derive(Deserialize)]
pub(crate) struct NuevaAnual {
	alumno_id: i32,
	anio: Option<i32>,
	monto_mxn: Option<i32>,
}

pub(crate) async fn guardar_anual(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevaAnual>,
) -> Result<HttpResponse, ApiError> {
	let monto = body.monto_mxn.unwrap_or(db::INSCRIPCION_ANUAL_MXN);
	exigir_monto(monto)?;
	let anio = match body.anio {
		Some(anio) => anio,
		None => db::anio_actual(db.get_ref()).await.map_err(ApiError::db)?,
	};
	let modelo = anual::ActiveModel {
		alumno_id: Set(body.alumno_id),
		anio: Set(anio),
		monto_mxn: Set(monto),
		..Default::default()
	};
	let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Created().json(creado))
}
