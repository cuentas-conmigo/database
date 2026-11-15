//! Lo que la app necesita y el esquema de Amanda aún no guarda (`migrations/0004_…`): teléfono y
//! ayudantes del alumno, foto con permiso, solicitudes de lugar y códigos de inscripción.

use sea_orm::{
	ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter, Set,
	Statement, Value,
};

use crate::models::dto::{HelperDto, PhotoDto, StudentDto};
use crate::models::entities::{alumno, inscripcion};

fn sql(texto: &str, valores: Vec<Value>) -> Statement {
	Statement::from_sql_and_values(DbBackend::Postgres, texto, valores)
}

/// Only the digits of a phone number.
pub fn digitos(texto: &str) -> String {
	texto.chars().filter(char::is_ascii_digit).collect()
}

/// The student as the app sees them: name, WhatsApp, helpers and a photo only with consent.
pub async fn perfil(conn: &DatabaseConnection, fila: alumno::Model) -> Result<StudentDto, DbErr> {
	let telefono = conn
		.query_one_raw(sql(
			"SELECT telefono FROM alumnos_contacto WHERE alumno_id = $1",
			vec![fila.id.into()],
		))
		.await?
		.map(|row| row.try_get::<String>("", "telefono"))
		.transpose()?
		.unwrap_or_default();
	let helpers = conn
		.query_all_raw(sql(
			"SELECT nombre, parentesco, telefono FROM ayudantes WHERE alumno_id = $1 ORDER BY id",
			vec![fila.id.into()],
		))
		.await?
		.into_iter()
		.map(|row| {
			Ok(HelperDto {
				name: row.try_get("", "nombre")?,
				relation: row.try_get("", "parentesco")?,
				phone: row.try_get("", "telefono")?,
			})
		})
		.collect::<Result<Vec<_>, DbErr>>()?;
	let photo = conn
		.query_one_raw(sql(
			"SELECT uri, otorgado_por, otorgado_en FROM fotos_alumno WHERE alumno_id = $1",
			vec![fila.id.into()],
		))
		.await?
		.map(|row| -> Result<PhotoDto, DbErr> {
			Ok(PhotoDto {
				uri: row.try_get("", "uri")?,
				granted_by: row.try_get("", "otorgado_por")?,
				granted_at: crate::services::cobros::iso(&row.try_get::<String>("", "otorgado_en")?),
			})
		})
		.transpose()?;
	Ok(StudentDto {
		id: fila.id.to_string(),
		name: fila.nombre,
		phone: telefono,
		photo,
		helpers,
	})
}

/// Whether a WhatsApp number already belongs to a student.
pub async fn telefono_registrado(conn: &DatabaseConnection, telefono: &str) -> Result<bool, DbErr> {
	Ok(
		conn
			.query_one_raw(sql(
				"SELECT 1 AS hay FROM alumnos_contacto WHERE telefono = $1",
				vec![telefono.into()],
			))
			.await?
			.is_some(),
	)
}

/// Saves the student's WhatsApp and, when given, the phone of whoever helps them pay.
pub async fn guardar_contacto(
	conn: &DatabaseConnection,
	alumno_id: i32,
	telefono: &str,
	ayudante: Option<&str>,
) -> Result<(), DbErr> {
	conn
		.execute_raw(sql(
			"INSERT INTO alumnos_contacto (alumno_id, telefono) VALUES ($1, $2)
			ON CONFLICT (alumno_id) DO UPDATE SET telefono = EXCLUDED.telefono",
			vec![alumno_id.into(), telefono.into()],
		))
		.await?;
	if let Some(ayudante) = ayudante {
		conn
			.execute_raw(sql(
				"INSERT INTO ayudantes (alumno_id, nombre, parentesco, telefono) VALUES ($1, 'Familiar', 'Familiar', $2)
				ON CONFLICT (alumno_id, telefono) DO NOTHING",
				vec![alumno_id.into(), ayudante.into()],
			))
			.await?;
	}
	Ok(())
}

/// First helper of the student, who gets the CoDi push charge.
pub async fn primer_ayudante(conn: &DatabaseConnection, alumno_id: i32) -> Result<Option<String>, DbErr> {
	conn
		.query_one_raw(sql(
			"SELECT telefono FROM ayudantes WHERE alumno_id = $1 ORDER BY id LIMIT 1",
			vec![alumno_id.into()],
		))
		.await?
		.map(|row| row.try_get::<String>("", "telefono"))
		.transpose()
}

/// Keeps the photo together with who allowed it.
pub async fn guardar_foto(
	conn: &DatabaseConnection,
	alumno_id: i32,
	uri: &str,
	otorgado_por: &str,
) -> Result<(), DbErr> {
	conn
		.execute_raw(sql(
			"INSERT INTO fotos_alumno (alumno_id, uri, otorgado_por) VALUES ($1, $2, $3)
			ON CONFLICT (alumno_id) DO UPDATE SET uri = EXCLUDED.uri, otorgado_por = EXCLUDED.otorgado_por,
				otorgado_en = ahora_texto()",
			vec![alumno_id.into(), uri.into(), otorgado_por.into()],
		))
		.await?;
	Ok(())
}

/// Deletes the photo and its consent record.
pub async fn quitar_foto(conn: &DatabaseConnection, alumno_id: i32) -> Result<(), DbErr> {
	conn
		.execute_raw(sql(
			"DELETE FROM fotos_alumno WHERE alumno_id = $1",
			vec![alumno_id.into()],
		))
		.await?;
	Ok(())
}

/// People asking for a place in the workshop, oldest first, with `pending` or `waitlist`.
pub async fn solicitudes(conn: &DatabaseConnection, taller_id: i32) -> Result<Vec<(alumno::Model, String)>, DbErr> {
	let filas = conn
		.query_all_raw(sql(
			"SELECT alumno_id, estado FROM solicitudes WHERE taller_id = $1 ORDER BY creado_en, alumno_id",
			vec![taller_id.into()],
		))
		.await?;
	let mut lista = Vec::with_capacity(filas.len());
	for fila in filas {
		let alumno_id: i32 = fila.try_get("", "alumno_id")?;
		if let Some(alumno) = alumno::Entity::find_by_id(alumno_id).one(conn).await? {
			lista.push((alumno, fila.try_get("", "estado")?));
		}
	}
	Ok(lista)
}

/// Removes a request; `false` when there was none.
pub async fn quitar_solicitud(conn: &DatabaseConnection, taller_id: i32, alumno_id: i32) -> Result<bool, DbErr> {
	let resultado = conn
		.execute_raw(sql(
			"DELETE FROM solicitudes WHERE taller_id = $1 AND alumno_id = $2",
			vec![taller_id.into(), alumno_id.into()],
		))
		.await?;
	Ok(resultado.rows_affected() > 0)
}

/// Turns a request into an active enrollment for this year; `false` when there was no request.
pub async fn aprobar(conn: &DatabaseConnection, taller_id: i32, alumno_id: i32) -> Result<bool, DbErr> {
	if !quitar_solicitud(conn, taller_id, alumno_id).await? {
		return Ok(false);
	}
	let anio: i32 = conn
		.query_one_raw(sql("SELECT anio_actual() AS anio", vec![]))
		.await?
		.ok_or_else(|| DbErr::Custom("anio_actual() returned nothing".into()))?
		.try_get("", "anio")?;
	let existente = inscripcion::Entity::find()
		.filter(inscripcion::Column::TallerId.eq(taller_id))
		.filter(inscripcion::Column::AlumnoId.eq(alumno_id))
		.filter(inscripcion::Column::Anio.eq(anio))
		.one(conn)
		.await?;
	match existente {
		Some(fila) => {
			let mut cambio: inscripcion::ActiveModel = fila.into();
			cambio.activa = Set(1);
			cambio.update(conn).await?;
		}
		None => {
			inscripcion::ActiveModel {
				alumno_id: Set(alumno_id),
				taller_id: Set(taller_id),
				anio: Set(anio),
				activa: Set(1),
				..Default::default()
			}
			.insert(conn)
			.await?;
		}
	}
	Ok(true)
}

/// The workshop's enrollment code, creating one the first time.
pub async fn codigo(conn: &DatabaseConnection, taller_id: i32, nombre: &str) -> Result<String, DbErr> {
	let actual = conn
		.query_one_raw(sql(
			"SELECT codigo FROM codigos_inscripcion WHERE taller_id = $1",
			vec![taller_id.into()],
		))
		.await?
		.map(|row| row.try_get::<String>("", "codigo"))
		.transpose()?;
	match actual {
		Some(codigo) => Ok(codigo),
		None => cambiar_codigo(conn, taller_id, nombre).await,
	}
}

/// Replaces the code with a new one (`TEJIDO-4821`); the old one stops working at once.
pub async fn cambiar_codigo(conn: &DatabaseConnection, taller_id: i32, nombre: &str) -> Result<String, DbErr> {
	let prefijo = prefijo(nombre);
	let semilla = chrono::Utc::now().timestamp_subsec_nanos();
	for intento in 0..20u32 {
		let numero = (semilla / 1000 + intento * 7919) % 9000 + 1000;
		let codigo = format!("{prefijo}-{numero}");
		let guardado = conn
			.execute_raw(sql(
				"INSERT INTO codigos_inscripcion (taller_id, codigo) VALUES ($1, $2)
				ON CONFLICT (taller_id) DO UPDATE SET codigo = EXCLUDED.codigo",
				vec![taller_id.into(), codigo.clone().into()],
			))
			.await;
		match guardado {
			Ok(_) => return Ok(codigo),
			Err(error) if error.to_string().contains("duplicate key") => continue,
			Err(error) => return Err(error),
		}
	}
	Err(DbErr::Custom("could not find a free enrollment code".into()))
}

/// Last word of the workshop's name, in capitals and without accents: "Taller de tejido" → "TEJIDO".
fn prefijo(nombre: &str) -> String {
	let palabra = nombre.split_whitespace().last().unwrap_or("TALLER");
	let limpia: String = palabra
		.chars()
		.map(|letra| match letra {
			'á' | 'Á' => 'A',
			'é' | 'É' => 'E',
			'í' | 'Í' => 'I',
			'ó' | 'Ó' => 'O',
			'ú' | 'Ú' | 'ü' | 'Ü' => 'U',
			'ñ' | 'Ñ' => 'N',
			otra => otra,
		})
		.filter(char::is_ascii_alphanumeric)
		.collect::<String>()
		.to_ascii_uppercase();
	if limpia.is_empty() {
		"TALLER".to_string()
	} else {
		limpia
	}
}

#[cfg(test)]
mod tests {
	use super::{digitos, prefijo};

	#[test]
	fn el_prefijo_es_la_ultima_palabra_sin_acentos() {
		assert_eq!(prefijo("Taller de tejido"), "TEJIDO");
		assert_eq!(prefijo("Yoga en silla"), "SILLA");
		assert_eq!(prefijo("Cerámica"), "CERAMICA");
		assert_eq!(prefijo(""), "TALLER");
	}

	#[test]
	fn solo_quedan_los_digitos_del_telefono() {
		assert_eq!(digitos("55 1234-5678"), "5512345678");
	}
}
