//! `GET /pagos`

use crate::error::ApiError;
use crate::models::entities::pago;
use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

#[get("")]
pub(crate) async fn listar_pagos(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let filas = pago::Entity::find()
		.order_by_asc(pago::Column::Id)
		.all(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(filas))
}
