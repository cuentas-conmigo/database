//! `PUT /alumnos/{id}`

use crate::error::ApiError;
use actix_web::{HttpResponse, put, web};
use sea_orm::DatabaseConnection;

use super::{CambioAlumno, guardar_cambio_alumno};

#[put("/{id}")]
pub(crate) async fn cambiar_alumno(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<CambioAlumno>,
) -> Result<HttpResponse, ApiError> {
	guardar_cambio_alumno(db, id, body).await
}
