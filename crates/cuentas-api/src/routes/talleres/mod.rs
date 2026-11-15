//! `/v1/talleres` — Talleres y cuotas (administración), y el taller del tallerista para la app.

mod _agregar_navegador;
mod _alumnos;
mod _aprobar;
mod _baja;
mod _borrar;
mod _borrar_navegador;
mod _cambiar_cuota;
mod _cobros;
mod _codigo_cambiar;
mod _codigo_ver;
mod _crear;
mod _cuota_navegador;
mod _inscribir_alumno;
mod _listar;
mod _solicitudes;
mod _ver;

use crate::error::ApiError;
use crate::models::entities::{cambio_cuota, taller};
use crate::routes::comun::*;
use crate::services as db;
use actix_web::{HttpResponse, web};
use sea_orm::{
	ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, Set, Statement, TransactionTrait,
};

use crate::error::AppResult;
use serde::Deserialize;

use _agregar_navegador::taller_agregar_navegador;
use _alumnos::alumnos_del_taller;
use _aprobar::aprobar_alumno;
use _baja::dar_de_baja;
use _borrar::borrar_taller;
use _borrar_navegador::taller_borrar_navegador;
use _cambiar_cuota::cambiar_cuota;
use _cobros::cobros_del_taller;
use _codigo_cambiar::cambiar_codigo;
use _codigo_ver::ver_codigo;
use _crear::crear_taller;
use _cuota_navegador::taller_cuota_navegador;
use _inscribir_alumno::inscribir_alumno;
use _listar::listar_talleres;
use _solicitudes::solicitudes;
use _ver::ver_taller;

/// Registra `/talleres` y sus acciones.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg.service(
		web::scope("/talleres")
			.service(listar_talleres)
			.service(crear_taller)
			.service(taller_agregar_navegador)
			.service(taller_borrar_navegador)
			.service(taller_cuota_navegador)
			.service(borrar_taller)
			.service(cambiar_cuota)
			// Rutas de la app: después de las de Amanda, para que `/agregar` y demás no caigan en `/{id}`.
			.service(ver_taller)
			.service(alumnos_del_taller)
			.service(cobros_del_taller)
			.service(solicitudes)
			.service(ver_codigo)
			.service(cambiar_codigo)
			.service(inscribir_alumno)
			.service(aprobar_alumno)
			.service(dar_de_baja),
	);
}

#[derive(Deserialize)]
pub(crate) struct NuevoTaller {
	tallerista_id: i32,
	nombre: String,
	cuota_mensual_mxn: Option<i32>,
}

pub(crate) async fn guardar_taller(
	db: web::Data<DatabaseConnection>,
	body: web::Json<NuevoTaller>,
) -> Result<HttpResponse, ApiError> {
	exigir_texto(&body.nombre, "nombre")?;
	let cuota = body.cuota_mensual_mxn.unwrap_or(db::MENSUALIDAD_MXN);
	exigir_monto(cuota)?;
	let txn = db.get_ref().begin().await.map_err(ApiError::db)?;
	let modelo = taller::ActiveModel {
		tallerista_id: Set(body.tallerista_id),
		nombre: Set(body.nombre.trim().to_string()),
		cuota_mensual_mxn: Set(cuota),
		activo: Set(1),
		..Default::default()
	};
	let creado = modelo.insert(&txn).await.map_err(ApiError::db)?;
	cambio_cuota::ActiveModel {
		taller_id: Set(creado.id),
		cuota_anterior_mxn: Set(None),
		cuota_nueva_mxn: Set(cuota),
		vigente_desde: Set(db::ahora(db.get_ref()).await.map_err(ApiError::db)?),
		..Default::default()
	}
	.insert(&txn)
	.await
	.map_err(ApiError::db)?;
	txn.commit().await.map_err(ApiError::db)?;
	Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
pub(crate) struct NuevaCuota {
	cuota_mensual_mxn: i32,
}

pub(crate) async fn guardar_cuota(
	db: web::Data<DatabaseConnection>,
	id: web::Path<i32>,
	body: web::Json<NuevaCuota>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	exigir_monto(body.cuota_mensual_mxn)?;
	let txn = db.get_ref().begin().await.map_err(ApiError::db)?;
	let Some(actual) = taller::Entity::find_by_id(id).one(&txn).await.map_err(ApiError::db)? else {
		return Err(ApiError::no("taller", id));
	};
	if actual.cuota_mensual_mxn == body.cuota_mensual_mxn {
		return Ok(HttpResponse::Ok().json(actual));
	}
	let anterior = actual.cuota_mensual_mxn;
	let mut activo: taller::ActiveModel = actual.into();
	activo.cuota_mensual_mxn = Set(body.cuota_mensual_mxn);
	let actualizado = activo.update(&txn).await.map_err(ApiError::db)?;
	cambio_cuota::ActiveModel {
		taller_id: Set(id),
		cuota_anterior_mxn: Set(Some(anterior)),
		cuota_nueva_mxn: Set(body.cuota_mensual_mxn),
		vigente_desde: Set(db::ahora(db.get_ref()).await.map_err(ApiError::db)?),
		..Default::default()
	}
	.insert(&txn)
	.await
	.map_err(ApiError::db)?;
	txn.commit().await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(actualizado))
}

#[derive(Deserialize)]
pub(crate) struct CuotaDireccion {
	id: i32,
	cuota_mensual_mxn: i32,
}

/// OpenAPI de las rutas de la app en este recurso.
#[derive(utoipa::OpenApi)]
#[openapi(paths(
	_ver::ver_taller,
	_alumnos::alumnos_del_taller,
	_cobros::cobros_del_taller,
	_solicitudes::solicitudes,
	_codigo_ver::ver_codigo,
	_codigo_cambiar::cambiar_codigo,
	_inscribir_alumno::inscribir_alumno,
	_aprobar::aprobar_alumno,
	_baja::dar_de_baja
))]
pub struct ApiDoc;

/// "Martes y jueves, 10:00" a partir de `horarios`.
pub(super) async fn horario_texto(conn: &DatabaseConnection, workshop_id: i32) -> AppResult<String> {
	let filas = conn
		.query_all_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT dia, hora_inicio FROM horarios WHERE taller_id = $1 ORDER BY id",
			[workshop_id.into()],
		))
		.await?;
	let mut days: Vec<String> = Vec::new();
	let mut time = None;
	for fila in filas {
		let dia: String = fila.try_get("", "dia")?;
		let dia = db::dia_visible(&dia).to_string();
		if !days.contains(&dia) {
			days.push(dia);
		}
		if time.is_none() {
			time = Some(fila.try_get::<String>("", "hora_inicio")?);
		}
	}
	let joined = match days.as_slice() {
		[] => return Ok(String::new()),
		[one] => one.clone(),
		[rest @ .., last] => format!("{} y {last}", rest.join(", ")),
	};
	let mut chars = joined.chars();
	let capitalized = chars
		.next()
		.map(|first| first.to_uppercase().chain(chars).collect::<String>())
		.unwrap_or_default();
	Ok(match time {
		Some(time) => format!("{capitalized}, {time}"),
		None => capitalized,
	})
}
