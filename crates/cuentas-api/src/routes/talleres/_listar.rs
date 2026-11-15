//! `GET /talleres`

use crate::error::ApiError;
use crate::models::entities::taller;
use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

#[get("")]
pub(crate) async fn listar_talleres(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = taller::Entity::find()
		.order_by_asc(taller::Column::Id)
		.all(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(filas))
}
