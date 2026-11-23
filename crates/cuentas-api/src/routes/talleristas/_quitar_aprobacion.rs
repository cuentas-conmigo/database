//! `DELETE /v1/talleristas/{id}/cobros-reales`

use crate::error::ApiError;
use crate::services::cobros::aprobacion;
use actix_web::{HttpResponse, delete, web};
use sea_orm::DatabaseConnection;

/// Takes back real CoDi charges; the account can still note cash.
#[delete("/{id}/cobros-reales")]
pub(crate) async fn quitar_cobros_reales(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	aprobacion::revoke(db.get_ref(), id).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(serde_json::json!({ "id": id, "cobrosReales": false })))
}
