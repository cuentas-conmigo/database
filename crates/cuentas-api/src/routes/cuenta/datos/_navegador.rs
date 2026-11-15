//! `GET /cuenta/datos`

use crate::error::ApiError;
use crate::services as db;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::DatosCuenta;

#[get("")]
pub(crate) async fn datos_cuenta_navegador(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	query: web::Query<DatosCuenta>,
) -> Result<HttpResponse, ApiError> {
	db::guardar_datos_cuenta(db.get_ref(), sesion.id, &query.nombre, &query.oficio)
		.await
		.map_err(ApiError::db)?;
	let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(cuerpo))
}
