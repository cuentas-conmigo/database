//! `/nfc` — Dispositivos NFC (administración).

mod _agregar_navegador;
mod _borrar;
mod _borrar_navegador;
mod _crear;
mod _listar;

use crate::error::ApiError;
use crate::models::entities::nfc;
use crate::routes::comun::*;
use actix_web::{HttpResponse, web};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde::Deserialize;

use _agregar_navegador::nfc_agregar_navegador;
use _borrar::borrar_nfc;
use _borrar_navegador::nfc_borrar_navegador;
use _crear::crear_nfc;
use _listar::listar_nfc;

/// Registra `/nfc` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/nfc")
			.service(listar_nfc)
			.service(crear_nfc)
			.service(nfc_agregar_navegador)
			.service(nfc_borrar_navegador)
			.service(borrar_nfc),
	);
}

#[derive(Deserialize)]
pub(crate) struct NuevoNfc {
	tallerista_id: i32,
	cara: String,
	chip_uid: String,
}

pub(crate) async fn guardar_nfc(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoNfc>,
) -> Result<HttpResponse, ApiError> {
	exigir_cara(&body.cara)?;
	exigir_texto(&body.chip_uid, "chip_uid")?;
	let modelo = nfc::ActiveModel {
		tallerista_id: Set(body.tallerista_id),
		cara: Set(body.cara.clone()),
		chip_uid: Set(body.chip_uid.trim().to_string()),
		..Default::default()
	};
	let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Created().json(creado))
}
