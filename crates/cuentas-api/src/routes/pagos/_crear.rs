//! `POST /pagos`

use crate::error::ApiError;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::{NuevoPago, guardar_pago};

#[post("")]
pub(crate) async fn crear_pago(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoPago>,
) -> Result<HttpResponse, ApiError> {
	guardar_pago(db, body).await
}
