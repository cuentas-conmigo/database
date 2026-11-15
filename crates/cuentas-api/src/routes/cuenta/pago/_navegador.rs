//! `GET /cuenta/pago`

use crate::error::ApiError;
use crate::services as db;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::PagoDeCuenta;

#[get("")]
pub(crate) async fn pago_cuenta_navegador(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	query: web::Query<PagoDeCuenta>,
) -> Result<HttpResponse, ApiError> {
	let estado = query.estado.as_deref().unwrap_or("pagado");
	db::marcar_pago_cuenta(db.get_ref(), sesion.id, &query.alumno, query.taller.as_deref(), estado)
		.await
		.map_err(ApiError::db)?;
	let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(cuerpo))
}
