//! `GET /ingresos`

use crate::error::ApiError;
use crate::services as db;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

#[get("")]
pub(crate) async fn listar_ingresos(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = db::ingresos(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(filas))
}
