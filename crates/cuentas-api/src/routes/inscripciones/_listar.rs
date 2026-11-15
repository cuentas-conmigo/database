//! `GET /inscripciones`

use crate::error::ApiError;
use crate::models::entities::inscripcion;
use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

#[get("")]
pub(crate) async fn listar_inscripciones(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = inscripcion::Entity::find()
		.order_by_asc(inscripcion::Column::Id)
		.all(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(filas))
}
