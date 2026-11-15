//! `GET /resumen`

use crate::error::ApiError;
use crate::models::entities::{alumno, inscripcion, pago, tallerista};
use crate::services as db;
use actix_web::{HttpResponse, get, web};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

#[get("")]
pub(crate) async fn resumen(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
	let ingresos = db::ingresos(db.get_ref()).await.map_err(ApiError::db)?;
	let pagos_pagados = pago::Entity::find()
		.filter(pago::Column::Estado.eq("pagado"))
		.count(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	let pagos_pendientes = pago::Entity::find()
		.filter(pago::Column::Estado.eq("pendiente"))
		.count(db.get_ref())
		.await
		.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(serde_json::json!({
			"inscripcion_anual_mxn": db::INSCRIPCION_ANUAL_MXN,
			"mensualidad_mxn": db::MENSUALIDAD_MXN,
			"talleristas": tallerista::Entity::find().count(db.get_ref()).await.map_err(ApiError::db)?,
			"alumnos": alumno::Entity::find().count(db.get_ref()).await.map_err(ApiError::db)?,
			"inscripciones": inscripcion::Entity::find().count(db.get_ref()).await.map_err(ApiError::db)?,
			"pagos_pagados": pagos_pagados,
			"pagos_pendientes": pagos_pendientes,
			"ingresos": ingresos,
	})))
}
