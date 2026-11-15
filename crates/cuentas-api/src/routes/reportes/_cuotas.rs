//! `GET /cuotas`

use crate::error::ApiError;
use crate::models::entities::cambio_cuota;
use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

#[get("")]
pub(crate) async fn listar_cuotas(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = cambio_cuota::Entity::find()
		.order_by_asc(cambio_cuota::Column::Id)
		.all(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(filas))
}
