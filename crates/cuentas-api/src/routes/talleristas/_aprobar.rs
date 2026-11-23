//! `PUT /v1/talleristas/{id}/cobros-reales`

use crate::error::ApiError;
use crate::services::cobros::aprobacion;
use actix_web::{HttpResponse, put, web};
use sea_orm::DatabaseConnection;

/// Lets the account start real CoDi charges.
#[put("/{id}/cobros-reales")]
pub(crate) async fn aprobar_cobros_reales(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	if !aprobacion::approve(db.get_ref(), id).await.map_err(ApiError::db)? {
		return Err(ApiError::no("tallerista", id));
	}
	Ok(HttpResponse::Ok().json(serde_json::json!({ "id": id, "cobrosReales": true })))
}
