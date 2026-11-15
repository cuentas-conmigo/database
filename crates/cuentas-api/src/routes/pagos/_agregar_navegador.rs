//! `GET /pagos/agregar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{NuevoPago, guardar_pago};

#[get("/agregar")]
pub(crate) async fn pago_agregar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<NuevoPago>,
) -> Result<HttpResponse, ApiError> {
	guardar_pago(db, web::Json(query.into_inner())).await
}
