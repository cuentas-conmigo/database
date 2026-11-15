//! `PUT /talleres/{id}/cuota`

use crate::error::ApiError;
use actix_web::{HttpResponse, put, web};
use sea_orm::DatabaseConnection;

use super::{NuevaCuota, guardar_cuota};

#[put("/{id}/cuota")]
pub(crate) async fn cambiar_cuota(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<NuevaCuota>,
) -> Result<HttpResponse, ApiError> {
	guardar_cuota(db, id, body).await
}
