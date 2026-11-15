//! `/v1/alumnos` — Alumnos (administración), y un alumno como lo ve la app.

mod _agregar_navegador;
mod _borrar;
mod _borrar_navegador;
mod _cambiar;
mod _cambiar_navegador;
mod _crear;
mod _listar;
mod _quitar_foto;
mod _tarjeta;
mod _unirse;
mod _ver;

use crate::error::ApiError;
use crate::models::entities::alumno;
use crate::routes::comun::*;
use actix_web::{HttpResponse, http::StatusCode, web};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::Deserialize;

use _agregar_navegador::alumno_agregar_navegador;
use _borrar::borrar_alumno;
use _borrar_navegador::alumno_borrar_navegador;
use _cambiar::cambiar_alumno;
use _cambiar_navegador::alumno_cambiar_navegador;
use _crear::crear_alumno;
use _listar::listar_alumnos;
use _quitar_foto::quitar_foto;
use _tarjeta::tarjeta;
use _unirse::unirse;
use _ver::ver_alumno;

/// Registra `/alumnos` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/alumnos")
			.service(listar_alumnos)
			.service(crear_alumno)
			.service(alumno_agregar_navegador)
			.service(alumno_borrar_navegador)
			.service(alumno_cambiar_navegador)
			.service(borrar_alumno)
			.service(cambiar_alumno)
			// Rutas de la app: después de las de Amanda, para que `/agregar` y demás no caigan en `/{id}`.
			.service(ver_alumno)
			.service(quitar_foto)
			.service(tarjeta)
			.service(unirse),
	);
}

#[derive(Deserialize)]
pub(crate) struct NuevoAlumno {
	nombre: String,
	cartera_ref: String,
}

pub(crate) async fn guardar_alumno(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoAlumno>,
) -> Result<HttpResponse, ApiError> {
	exigir_texto(&body.nombre, "nombre")?;
	exigir_texto(&body.cartera_ref, "cartera_ref")?;
	let modelo = alumno::ActiveModel {
		nombre: Set(body.nombre.trim().to_string()),
		cartera_ref: Set(body.cartera_ref.trim().to_string()),
		..Default::default()
	};
	let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
pub(crate) struct CambioAlumno {
	nombre: Option<String>,
	cartera_ref: Option<String>,
}

pub(crate) async fn guardar_cambio_alumno(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<CambioAlumno>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	let Some(actual) = alumno::Entity::find_by_id(id)
		.one(db.get_ref())
		.await
		.map_err(ApiError::db)?
	else {
		return Err(ApiError::no("alumno", id));
	};
	let mut activo: alumno::ActiveModel = actual.into();
	if let Some(nombre) = &body.nombre {
		exigir_texto(nombre, "nombre")?;
		activo.nombre = Set(nombre.trim().to_string());
	}
	if let Some(cartera) = &body.cartera_ref {
		exigir_texto(cartera, "cartera_ref")?;
		activo.cartera_ref = Set(cartera.trim().to_string());
	}
	if body.nombre.is_none() && body.cartera_ref.is_none() {
		return Err(ApiError::mensaje(
			StatusCode::BAD_REQUEST,
			"indica nombre o cartera_ref",
		));
	}
	let actualizado = activo.update(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(actualizado))
}

#[derive(Deserialize)]
pub(crate) struct CambioAlumnoDireccion {
	id: i32,
	nombre: Option<String>,
	cartera_ref: Option<String>,
}

/// OpenAPI de las rutas de la app en este recurso.
#[derive(utoipa::OpenApi)]
#[openapi(paths(_ver::ver_alumno, _quitar_foto::quitar_foto, _tarjeta::tarjeta, _unirse::unirse))]
pub struct ApiDoc;
