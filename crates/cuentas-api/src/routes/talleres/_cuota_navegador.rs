//! `GET /talleres/cuota`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{CuotaDireccion, NuevaCuota, guardar_cuota};

#[get("/cuota")]
pub(crate) async fn taller_cuota_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<CuotaDireccion>,
) -> Result<HttpResponse, ApiError> {
	let datos = query.into_inner();
	guardar_cuota(
		db,
		web::Path::from(datos.id),
		web::Json(NuevaCuota {
			cuota_mensual_mxn: datos.cuota_mensual_mxn,
		}),
	)
	.await
}
