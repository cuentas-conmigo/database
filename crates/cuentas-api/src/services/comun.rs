//! Constantes y ayudas compartidas por los servicios.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

pub const INSCRIPCION_ANUAL_MXN: i32 = 125;

pub const MENSUALIDAD_MXN: i32 = 25;

pub async fn ahora<C>(db: &C) -> Result<String, DbErr>
where
	C: ConnectionTrait,
{
	let fila = db
		.query_one_raw(Statement::from_string(
			DbBackend::Postgres,
			"SELECT ahora_texto() AS ahora".to_owned(),
		))
		.await?
		.ok_or_else(|| DbErr::Custom("no se pudo leer la fecha".to_owned()))?;
	fila.try_get("", "ahora")
}

pub async fn anio_actual(db: &DatabaseConnection) -> Result<i32, DbErr> {
	let fila = db
		.query_one_raw(Statement::from_string(
			DbBackend::Postgres,
			"SELECT anio_actual() AS anio".to_owned(),
		))
		.await?
		.ok_or_else(|| DbErr::Custom("no se pudo leer el año".to_owned()))?;
	fila.try_get("", "anio")
}

pub(crate) fn dia_visible(dia: &str) -> &str {
	match dia {
		"miercoles" => "miércoles",
		"sabado" => "sábado",
		otro => otro,
	}
}

pub(crate) fn aviso(mensaje: impl Into<String>) -> DbErr {
	DbErr::Custom(format!("aviso: {}", mensaje.into()))
}

pub(crate) fn url_parte(valor: &str) -> String {
	valor.replace(' ', "%20")
}

pub(crate) fn marca() -> u128 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|duracion| duracion.as_millis())
		.unwrap_or(0)
}
