//! `GET /talleristas/cambiar`

use crate::error::ApiError;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::{CambioTallerista, CambioTalleristaDireccion, guardar_cambio_tallerista};

#[get("/cambiar")]
pub(crate) async fn tallerista_cambiar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<CambioTalleristaDireccion>,
) -> Result<HttpResponse, ApiError> {
	let datos = query.into_inner();
	guardar_cambio_tallerista(
		db,
		web::Path::from(datos.id),
		web::Json(CambioTallerista {
			nombre: datos.nombre,
			oficio: datos.oficio,
			pey_api_key_id: datos.pey_api_key_id,
		}),
	)
	.await
}
