//! `DELETE /inscripciones/{id}`

use crate::error::ApiError;
use crate::models::entities::inscripcion;
use crate::routes::comun::*;
use actix_web::{HttpResponse, delete, web};
use sea_orm::DatabaseConnection;

#[delete("/{id}")]
pub(crate) async fn borrar_inscripcion(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
	eliminar::<inscripcion::Entity>(db.get_ref(), id.into_inner(), "inscripcion").await
}
