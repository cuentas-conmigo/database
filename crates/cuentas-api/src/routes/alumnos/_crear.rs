//! `POST /alumnos`

use crate::error::ApiError;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::{NuevoAlumno, guardar_alumno};

#[post("")]
pub(crate) async fn crear_alumno(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoAlumno>,
) -> Result<HttpResponse, ApiError> {
	guardar_alumno(db, body).await
}
