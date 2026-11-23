//! Accounts the app creates and deletes: an instructor with their first workshop, and everything
//! tied to them when they leave.

use sea_orm::{
	ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, Statement, TransactionTrait,
};

use super::*;

/// What the app's sign-up form sends.
pub struct Registro<'a> {
	pub usuario: &'a str,
	pub clave: &'a str,
	pub nombre: &'a str,
	pub oficio: &'a str,
	pub taller: &'a str,
	pub cuota: i32,
}

#[derive(Debug)]
pub enum RegistroError {
	/// Someone already has that username.
	Taken,
	/// A field the instructor has to fix, in plain words.
	Invalid(String),
	Db(DbErr),
}

impl From<DbErr> for RegistroError {
	fn from(error: DbErr) -> Self {
		Self::Db(error)
	}
}

fn invalido(error: DbErr) -> RegistroError {
	let texto = error.to_string();
	match texto.split_once("aviso: ") {
		Some((_, aviso)) => RegistroError::Invalid(aviso.to_string()),
		None => RegistroError::Db(error),
	}
}

fn exigir(valor: &str, aviso: &str) -> Result<String, RegistroError> {
	let valor = valor.trim();
	if valor.is_empty() || valor.chars().count() > 80 {
		return Err(RegistroError::Invalid(aviso.to_string()));
	}
	Ok(valor.to_string())
}

/// A new account and the id of its first workshop.
pub struct Registrado {
	pub sesion: Sesion,
	pub taller_id: i32,
}

/// Creates the account and its first workshop together, so the app can open it right away.
pub async fn registrar(db: &DatabaseConnection, registro: Registro<'_>) -> Result<Registrado, RegistroError> {
	let usuario = usuario_valido(registro.usuario).map_err(invalido)?;
	let clave = clave_valida(registro.clave).map_err(invalido)?;
	let nombre = exigir(registro.nombre, "escribe tu nombre")?;
	let oficio = exigir(registro.oficio, "escribe qué enseñas")?;
	let taller = exigir(registro.taller, "escribe el nombre de tu taller")?;
	if !(1..=100_000).contains(&registro.cuota) {
		return Err(RegistroError::Invalid("la cuota mensual debe ser mayor que 0".into()));
	}
	let hash = crate::middleware::auth::hash_clave(clave)?;

	let txn = db.begin().await?;
	let creada = txn
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO talleristas (nombre, oficio, pey_api_key_id, usuario, clave_hash)
			 VALUES ($1, $2, $3, $4, $5)
			 ON CONFLICT DO NOTHING
			 RETURNING id",
			[
				nombre.clone().into(),
				oficio.into(),
				format!("ejemplo_pey_{usuario}").into(),
				usuario.clone().into(),
				hash.into(),
			],
		))
		.await?;
	let Some(creada) = creada else {
		return Err(RegistroError::Taken);
	};
	let id: i32 = creada.try_get("", "id")?;
	let taller_id: i32 = txn
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO talleres (tallerista_id, nombre, cuota_mensual_mxn) VALUES ($1, $2, $3) RETURNING id",
			[id.into(), taller.into(), registro.cuota.into()],
		))
		.await?
		.ok_or_else(|| DbErr::Custom("no se pudo crear el taller".to_owned()))?
		.try_get("", "id")?;
	let ahora = ahora(&txn).await?;
	txn
		.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde)
			 VALUES ($1, NULL, $2, $3)",
			[taller_id.into(), registro.cuota.into(), ahora.into()],
		))
		.await?;
	txn.commit().await?;
	Ok(Registrado {
		sesion: Sesion { id, nombre, usuario },
		taller_id,
	})
}

/// Deletes the instructor, their workshops with everything in them, and the students who were only
/// theirs. Students who also belong to someone else's workshop stay, with that workshop's records.
pub async fn borrar_cuenta(db: &DatabaseConnection, tallerista_id: i32) -> Result<(), DbErr> {
	let txn = db.begin().await?;
	let alumnos = alumnos_de(&txn, tallerista_id).await?;
	for sql in [
		"DELETE FROM pagos WHERE tallerista_id = $1
		 OR taller_id IN (SELECT id FROM talleres WHERE tallerista_id = $1)",
		"DELETE FROM inscripciones_taller WHERE taller_id IN (SELECT id FROM talleres WHERE tallerista_id = $1)",
		"DELETE FROM horarios WHERE taller_id IN (SELECT id FROM talleres WHERE tallerista_id = $1)",
		"DELETE FROM cambios_cuota WHERE taller_id IN (SELECT id FROM talleres WHERE tallerista_id = $1)",
		"DELETE FROM talleres WHERE tallerista_id = $1",
		"DELETE FROM dispositivos_nfc WHERE tallerista_id = $1",
		"DELETE FROM sesiones WHERE tallerista_id = $1",
		"DELETE FROM codigos_acceso WHERE tallerista_id = $1",
		"DELETE FROM talleristas WHERE id = $1",
	] {
		txn
			.execute_raw(Statement::from_sql_and_values(
				DbBackend::Postgres,
				sql,
				[tallerista_id.into()],
			))
			.await?;
	}
	if !alumnos.is_empty() {
		let ids = alumnos.iter().map(i32::to_string).collect::<Vec<_>>().join(", ");
		let solos = format!(
			"SELECT a.id FROM alumnos a WHERE a.id IN ({ids})
			 AND NOT EXISTS (SELECT 1 FROM inscripciones_taller i WHERE i.alumno_id = a.id)
			 AND NOT EXISTS (SELECT 1 FROM solicitudes s WHERE s.alumno_id = a.id)
			 AND NOT EXISTS (SELECT 1 FROM pagos p WHERE p.alumno_id = a.id)"
		);
		for sql in [
			format!("DELETE FROM inscripciones_anuales WHERE alumno_id IN ({solos})"),
			format!("DELETE FROM alumnos WHERE id IN ({solos})"),
		] {
			txn.execute_unprepared(&sql).await?;
		}
	}
	txn.commit().await
}

/// Students in the instructor's workshops: enrolled, waiting or with a payment there.
async fn alumnos_de(txn: &DatabaseTransaction, tallerista_id: i32) -> Result<Vec<i32>, DbErr> {
	let filas = txn
		.query_all_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT i.alumno_id AS id FROM inscripciones_taller i JOIN talleres t ON t.id = i.taller_id
			 WHERE t.tallerista_id = $1
			 UNION SELECT s.alumno_id FROM solicitudes s JOIN talleres t ON t.id = s.taller_id
			 WHERE t.tallerista_id = $1
			 UNION SELECT p.alumno_id FROM pagos p
			 WHERE p.tallerista_id = $1 OR p.taller_id IN (SELECT id FROM talleres WHERE tallerista_id = $1)",
			[tallerista_id.into()],
		))
		.await?;
	filas.iter().map(|fila| fila.try_get("", "id")).collect()
}
