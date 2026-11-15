//! `DELETE /alumnos/{id}`

use crate::error::ApiError;
use crate::models::entities::alumno;
use crate::routes::comun::*;
use actix_web::{HttpResponse, delete, web};
use sea_orm::DatabaseConnection;

#[delete("/{id}")]
pub(crate) async fn borrar_alumno(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
	eliminar::<alumno::Entity>(db.get_ref(), id.into_inner(), "alumno").await
}
