//! `GET /pagos/cambiar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{CambioPago, CambioPagoDireccion, guardar_cambio_pago};

#[get("/cambiar")]
pub(crate) async fn pago_cambiar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<CambioPagoDireccion>,
) -> Result<HttpResponse, ApiError> {
	let datos = query.into_inner();
	guardar_cambio_pago(
		db,
		web::Path::from(datos.id),
		web::Json(CambioPago {
			estado: datos.estado,
			monto_mxn: datos.monto_mxn,
		}),
	)
	.await
}
