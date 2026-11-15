//! `GET /nfc`

use crate::error::ApiError;
use crate::models::entities::nfc;
use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

#[get("")]
pub(crate) async fn listar_nfc(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = nfc::Entity::find()
		.order_by_asc(nfc::Column::Id)
		.all(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(filas))
}
