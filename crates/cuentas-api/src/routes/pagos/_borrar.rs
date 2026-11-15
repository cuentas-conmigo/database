//! `DELETE /pagos/{id}`

use crate::error::ApiError;
use crate::models::entities::pago;
use crate::routes::comun::*;
use actix_web::{HttpResponse, delete, web};
use sea_orm::DatabaseConnection;

#[delete("/{id}")]
pub(crate) async fn borrar_pago(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
	eliminar::<pago::Entity>(db.get_ref(), id.into_inner(), "pago").await
}
