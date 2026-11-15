//! `GET /alumnos`

use crate::error::ApiError;
use crate::models::entities::alumno;
use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

#[get("")]
pub(crate) async fn listar_alumnos(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = alumno::Entity::find()
		.order_by_asc(alumno::Column::Id)
		.all(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(filas))
}
