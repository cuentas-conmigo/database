//! `GET /cuenta/alumno`

use crate::error::ApiError;
use crate::services as db;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::AlumnoDeCuenta;

#[get("")]
pub(crate) async fn alumno_cuenta_navegador(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	query: web::Query<AlumnoDeCuenta>,
) -> Result<HttpResponse, ApiError> {
	db::guardar_alumno_cuenta(db.get_ref(), sesion.id, &query.nombre, query.taller.as_deref())
		.await
		.map_err(ApiError::db)?;
	let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(cuerpo))
}
