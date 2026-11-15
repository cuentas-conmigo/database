//! `GET /talleristas`

use crate::error::ApiError;
use crate::models::entities::tallerista;
use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

#[get("")]
pub(crate) async fn listar_talleristas(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = tallerista::Entity::find()
		.order_by_asc(tallerista::Column::Id)
		.all(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(filas))
}
