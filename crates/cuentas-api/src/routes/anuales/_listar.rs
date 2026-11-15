//! `GET /anuales`

use crate::error::ApiError;
use crate::models::entities::anual;
use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

#[get("")]
pub(crate) async fn listar_anuales(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = anual::Entity::find()
		.order_by_asc(anual::Column::Id)
		.all(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(filas))
}
