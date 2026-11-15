//! `/talleristas` — Talleristas (administración).

mod _agregar_navegador;
mod _borrar;
mod _borrar_navegador;
mod _cambiar;
mod _cambiar_navegador;
mod _crear;
mod _listar;

use crate::error::ApiError;
use crate::models::entities::tallerista;
use crate::routes::comun::*;
use actix_web::{HttpResponse, http::StatusCode, web};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::Deserialize;

use _agregar_navegador::tallerista_agregar_navegador;
use _borrar::borrar_tallerista;
use _borrar_navegador::tallerista_borrar_navegador;
use _cambiar::cambiar_tallerista;
use _cambiar_navegador::tallerista_cambiar_navegador;
use _crear::crear_tallerista;
use _listar::listar_talleristas;

/// Registra `/talleristas` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/talleristas")
			.service(listar_talleristas)
			.service(crear_tallerista)
			.service(tallerista_agregar_navegador)
			.service(tallerista_borrar_navegador)
			.service(tallerista_cambiar_navegador)
			.service(borrar_tallerista)
			.service(cambiar_tallerista),
	);
}

#[derive(Deserialize)]
pub(crate) struct NuevoTallerista {
	nombre: String,
	oficio: String,
	pey_api_key_id: String,
}

pub(crate) async fn guardar_tallerista(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoTallerista>,
) -> Result<HttpResponse, ApiError> {
	exigir_texto(&body.nombre, "nombre")?;
	exigir_texto(&body.oficio, "oficio")?;
	exigir_texto(&body.pey_api_key_id, "pey_api_key_id")?;
	let modelo = tallerista::ActiveModel {
		nombre: Set(body.nombre.trim().to_string()),
		oficio: Set(body.oficio.trim().to_string()),
		pey_api_key_id: Set(body.pey_api_key_id.trim().to_string()),
		..Default::default()
	};
	let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
pub(crate) struct CambioTallerista {
	nombre: Option<String>,
	oficio: Option<String>,
	pey_api_key_id: Option<String>,
}

pub(crate) async fn guardar_cambio_tallerista(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<CambioTallerista>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	let Some(actual) = tallerista::Entity::find_by_id(id)
		.one(db.get_ref())
		.await
		.map_err(ApiError::db)?
	else {
		return Err(ApiError::no("tallerista", id));
	};
	if body.nombre.is_none() && body.oficio.is_none() && body.pey_api_key_id.is_none() {
		return Err(ApiError::mensaje(
			StatusCode::BAD_REQUEST,
			"indica nombre, oficio o pey_api_key_id",
		));
	}
	let mut activo: tallerista::ActiveModel = actual.into();
	if let Some(nombre) = &body.nombre {
		exigir_texto(nombre, "nombre")?;
		activo.nombre = Set(nombre.trim().to_string());
	}
	if let Some(oficio) = &body.oficio {
		exigir_texto(oficio, "oficio")?;
		activo.oficio = Set(oficio.trim().to_string());
	}
	if let Some(pey) = &body.pey_api_key_id {
		exigir_texto(pey, "pey_api_key_id")?;
		activo.pey_api_key_id = Set(pey.trim().to_string());
	}
	let actualizado = activo.update(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(actualizado))
}

#[derive(Deserialize)]
pub(crate) struct CambioTalleristaDireccion {
	id: i32,
	nombre: Option<String>,
	oficio: Option<String>,
	pey_api_key_id: Option<String>,
}
