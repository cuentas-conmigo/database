//! `PUT /talleristas/{id}`

use crate::error::ApiError;
use actix_web::{HttpResponse, put, web};
use sea_orm::DatabaseConnection;

use super::{CambioTallerista, guardar_cambio_tallerista};

#[put("/{id}")]
pub(crate) async fn cambiar_tallerista(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<CambioTallerista>,
) -> Result<HttpResponse, ApiError> {
	guardar_cambio_tallerista(db, id, body).await
}
