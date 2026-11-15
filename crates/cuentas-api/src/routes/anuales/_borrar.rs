//! `DELETE /anuales/{id}`

use crate::error::ApiError;
use crate::models::entities::anual;
use crate::routes::comun::*;
use actix_web::{HttpResponse, delete, web};
use sea_orm::DatabaseConnection;

#[delete("/{id}")]
pub(crate) async fn borrar_anual(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
	eliminar::<anual::Entity>(db.get_ref(), id.into_inner(), "inscripcion anual").await
}
