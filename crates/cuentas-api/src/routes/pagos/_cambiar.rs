//! `PUT /pagos/{id}`

use crate::error::ApiError;
use actix_web::{HttpResponse, put, web};
use sea_orm::DatabaseConnection;

use super::{CambioPago, guardar_cambio_pago};

#[put("/{id}")]
pub(crate) async fn cambiar_pago(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<CambioPago>,
) -> Result<HttpResponse, ApiError> {
	guardar_cambio_pago(db, id, body).await
}
