//! `GET /talleres/agregar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{NuevoTaller, guardar_taller};

#[get("/agregar")]
pub(crate) async fn taller_agregar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<NuevoTaller>,
) -> Result<HttpResponse, ApiError> {
	guardar_taller(db, web::Json(query.into_inner())).await
}
