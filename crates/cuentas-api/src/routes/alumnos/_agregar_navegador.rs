//! `GET /alumnos/agregar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{NuevoAlumno, guardar_alumno};

#[get("/agregar")]
pub(crate) async fn alumno_agregar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<NuevoAlumno>,
) -> Result<HttpResponse, ApiError> {
	guardar_alumno(db, web::Json(query.into_inner())).await
}
