//! `/v1/pagos` — Pagos (administración), y los cobros por CoDi o en efectivo de la app.

mod _agregar_navegador;
mod _borrar;
mod _borrar_navegador;
mod _cambiar;
mod _cambiar_navegador;
mod _cobrar;
mod _crear;
mod _listar;
mod _ver;

use crate::error::ApiError;
use crate::models::entities::{pago, taller};
use crate::routes::comun::*;
use crate::services as db;
use actix_web::{HttpResponse, http::StatusCode, web};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::Deserialize;

use _agregar_navegador::pago_agregar_navegador;
use _borrar::borrar_pago;
use _borrar_navegador::pago_borrar_navegador;
use _cambiar::cambiar_pago;
use _cambiar_navegador::pago_cambiar_navegador;
use _cobrar::cobrar;
use _crear::crear_pago;
use _listar::listar_pagos;
use _ver::ver_pago;

/// Registra `/pagos` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/pagos")
			.service(listar_pagos)
			.service(crear_pago)
			.service(pago_agregar_navegador)
			.service(pago_borrar_navegador)
			.service(pago_cambiar_navegador)
			.service(borrar_pago)
			.service(cambiar_pago)
			// Rutas de la app: después de las de Amanda, para que `/agregar` y demás no caigan en `/{id}`.
			.service(cobrar)
			.service(ver_pago),
	);
}

#[derive(Deserialize)]
pub(crate) struct NuevoPago {
	alumno_id: i32,
	tipo: String,
	canal: String,
	periodo: String,
	taller_id: Option<i32>,
	monto_mxn: Option<i32>,
	num_ref: Option<String>,
	estado: Option<String>,
}

pub(crate) async fn guardar_pago(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoPago>,
) -> Result<HttpResponse, ApiError> {
	exigir_tipo(&body.tipo)?;
	exigir_canal(&body.canal)?;
	exigir_texto(&body.periodo, "periodo")?;
	let estado = body.estado.clone().unwrap_or_else(|| "pendiente".to_string());
	exigir_estado(&estado)?;

	let (taller_id, tallerista_id, concepto, cuota) = if body.tipo == "mensualidad" {
		let taller_id = body
			.taller_id
			.ok_or_else(|| ApiError::mensaje(StatusCode::BAD_REQUEST, "la mensualidad necesita taller_id"))?;
		let Some(taller) = taller::Entity::find_by_id(taller_id)
			.one(db.get_ref())
			.await
			.map_err(ApiError::db)?
		else {
			return Err(ApiError::no("taller", taller_id));
		};
		(
			Some(taller_id),
			Some(taller.tallerista_id),
			format!("Mensualidad {} {}", taller.nombre, body.periodo.trim()),
			taller.cuota_mensual_mxn,
		)
	} else if body.taller_id.is_some() {
		return Err(ApiError::mensaje(
			StatusCode::BAD_REQUEST,
			"la inscripcion anual no lleva taller_id",
		));
	} else {
		(
			None,
			None,
			format!("Inscripción anual {}", body.periodo.trim()),
			db::INSCRIPCION_ANUAL_MXN,
		)
	};

	let monto = body.monto_mxn.unwrap_or(cuota);
	exigir_monto(monto)?;
	let num_ref = body
		.num_ref
		.as_deref()
		.map(str::trim)
		.filter(|valor| !valor.is_empty())
		.map(str::to_string)
		.unwrap_or_else(|| {
			format!(
				"manual-{}-{}-{}-{}",
				body.tipo,
				body.alumno_id,
				body.periodo.trim(),
				marca_ms()
			)
		});
	let (transaction_id, tipo_evento, pagado_en) = if estado == "pagado" {
		(
			Some(format!("txn-{num_ref}")),
			Some("codi.qr.payment_received".to_string()),
			Some(db::ahora(db.get_ref()).await.map_err(ApiError::db)?),
		)
	} else {
		(None, None, None)
	};
	let modelo = pago::ActiveModel {
		alumno_id: Set(body.alumno_id),
		taller_id: Set(taller_id),
		tallerista_id: Set(tallerista_id),
		tipo: Set(body.tipo.clone()),
		canal: Set(body.canal.clone()),
		concepto: Set(concepto),
		monto_mxn: Set(monto),
		num_ref: Set(num_ref.clone()),
		codi_qr_id: Set(Some(format!("qr-{num_ref}"))),
		transaction_id: Set(transaction_id),
		tipo_evento: Set(tipo_evento),
		estado: Set(estado),
		periodo: Set(body.periodo.trim().to_string()),
		pagado_en: Set(pagado_en),
		..Default::default()
	};
	let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
pub(crate) struct CambioPago {
	estado: Option<String>,
	monto_mxn: Option<i32>,
}

pub(crate) async fn guardar_cambio_pago(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<CambioPago>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	if body.estado.is_none() && body.monto_mxn.is_none() {
		return Err(ApiError::mensaje(StatusCode::BAD_REQUEST, "indica estado o monto_mxn"));
	}
	if let Some(estado) = &body.estado {
		exigir_estado(estado)?;
	}
	if let Some(monto) = body.monto_mxn {
		exigir_monto(monto)?;
	}
	let Some(actual) = pago::Entity::find_by_id(id)
		.one(db.get_ref())
		.await
		.map_err(ApiError::db)?
	else {
		return Err(ApiError::no("pago", id));
	};
	let estado = body.estado.clone().unwrap_or(actual.estado.clone());
	let monto = body.monto_mxn.unwrap_or(actual.monto_mxn);
	let mut transaction_id = actual.transaction_id.clone();
	let mut tipo_evento = actual.tipo_evento.clone();
	let mut pagado_en = actual.pagado_en.clone();
	if estado == "pagado" {
		if transaction_id.is_none() {
			transaction_id = Some(format!("txn-manual-{id}"));
		}
		if tipo_evento.is_none() {
			tipo_evento = Some("codi.qr.payment_received".to_string());
		}
		if pagado_en.is_none() {
			pagado_en = Some(db::ahora(db.get_ref()).await.map_err(ApiError::db)?);
		}
	} else {
		pagado_en = None;
		tipo_evento = None;
	}
	let mut activo: pago::ActiveModel = actual.into();
	activo.estado = Set(estado);
	activo.monto_mxn = Set(monto);
	activo.transaction_id = Set(transaction_id);
	activo.tipo_evento = Set(tipo_evento);
	activo.pagado_en = Set(pagado_en);
	let actualizado = activo.update(db.get_ref()).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(actualizado))
}

#[derive(Deserialize)]
pub(crate) struct CambioPagoDireccion {
	id: i32,
	estado: Option<String>,
	monto_mxn: Option<i32>,
}

/// OpenAPI de las rutas de la app en este recurso.
#[derive(utoipa::OpenApi)]
#[openapi(paths(_cobrar::cobrar, _ver::ver_pago))]
pub struct ApiDoc;
