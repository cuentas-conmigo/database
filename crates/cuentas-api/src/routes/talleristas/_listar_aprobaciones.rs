//! `GET /v1/talleristas/cobros-reales`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Cuenta {
	id: i32,
	nombre: String,
	usuario: Option<String>,
	aprobado_en: Option<String>,
}

/// Every account and whether it may start real CoDi charges, waiting ones first.
#[get("/cobros-reales")]
pub(crate) async fn listar_cobros_reales(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = db
		.query_all_raw(Statement::from_string(
			DbBackend::Postgres,
			"SELECT t.id, t.nombre, t.usuario, c.aprobado_en
			 FROM talleristas t LEFT JOIN cobros_reales c ON c.tallerista_id = t.id
			 ORDER BY c.aprobado_en IS NOT NULL, t.id",
		))
		.await
		.map_err(ApiError::db)?;
	let cuentas = filas
		.iter()
		.map(|fila| {
			Ok(Cuenta {
				id: fila.try_get("", "id")?,
				nombre: fila.try_get("", "nombre")?,
				usuario: fila.try_get("", "usuario")?,
				aprobado_en: fila.try_get("", "aprobado_en")?,
			})
		})
		.collect::<Result<Vec<_>, sea_orm::DbErr>>()
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(cuentas))
}
