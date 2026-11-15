//! `GET /pagos/borrar`

use crate::error::ApiError;
use crate::models::entities::pago;
use crate::routes::comun::*;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

#[get("/borrar")]
pub(crate) async fn pago_borrar_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
	borrar_en_navegador::<pago::Entity>(db.get_ref(), query.id, "pago").await
}
