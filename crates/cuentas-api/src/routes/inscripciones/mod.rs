//! `/inscripciones` — Inscripciones a talleres (administración).

mod _agregar_navegador;
mod _borrar;
mod _borrar_navegador;
mod _cambiar;
mod _cambiar_navegador;
mod _crear;
mod _listar;

use crate::error::ApiError;
use crate::models::entities::inscripcion;
use crate::services as db;
use actix_web::{HttpResponse, http::StatusCode, web};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::Deserialize;

use _agregar_navegador::inscripcion_agregar_navegador;
use _borrar::borrar_inscripcion;
use _borrar_navegador::inscripcion_borrar_navegador;
use _cambiar::cambiar_inscripcion;
use _cambiar_navegador::inscripcion_cambiar_navegador;
use _crear::crear_inscripcion;
use _listar::listar_inscripciones;

/// Registra `/inscripciones` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/inscripciones")
			.service(listar_inscripciones)
			.service(crear_inscripcion)
			.service(inscripcion_agregar_navegador)
			.service(inscripcion_borrar_navegador)
			.service(inscripcion_cambiar_navegador)
			.service(borrar_inscripcion)
			.service(cambiar_inscripcion),
	);
}

#[derive(Deserialize)]
pub(crate) struct NuevaInscripcion {
	alumno_id: i32,
	taller_id: i32,
	anio: Option<i32>,
}

pub(crate) async fn guardar_inscripcion(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevaInscripcion>,
) -> Result<HttpResponse, ApiError> {
	let anio = match body.anio {
		Some(anio) => anio,
		None => db::anio_actual(db.get_ref()).await.map_err(ApiError::db)?,
	};
	let modelo = inscripcion::ActiveModel {
		alumno_id: Set(body.alumno_id),
		taller_id: Set(body.taller_id),
		anio: Set(anio),
		activa: Set(1),
		..Default::default()
	};
	let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
pub(crate) struct CambioInscripcion {
	activa: i32,
}

pub(crate) async fn guardar_cambio_inscripcion(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<CambioInscripcion>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	if body.activa != 0 && body.activa != 1 {
		return Err(ApiError::mensaje(StatusCode::BAD_REQUEST, "activa debe ser 0 o 1"));
	}
	let Some(actual) = inscripcion::Entity::find_by_id(id)
		.one(db.get_ref())
		.await
		.map_err(ApiError::db)?
	else {
		return Err(ApiError::no("inscripcion", id));
	};
	let mut activo: inscripcion::ActiveModel = actual.into();
	activo.activa = Set(body.activa);
	let actualizado = activo.update(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(actualizado))
}

#[derive(Deserialize)]
pub(crate) struct CambioInscripcionDireccion {
	id: i32,
	activa: i32,
}
