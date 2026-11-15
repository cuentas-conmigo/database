//! Talleres, horarios y cuotas de la cuenta.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait};

use super::*;

pub async fn guardar_taller_cuenta(
	db: &DatabaseConnection,
	tallerista_id: i32,
	nombre: &str,
	cuota: i32,
) -> Result<(), DbErr> {
	let nombre = nombre.trim();
	if nombre.is_empty() {
		return Err(aviso("escribe el nombre del taller"));
	}
	if cuota <= 0 {
		return Err(aviso("la cuota debe ser mayor que 0"));
	}
	let txn = db.begin().await?;
	let actual = txn
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id, cuota_mensual_mxn FROM talleres WHERE tallerista_id = $1 AND nombre = $2",
			[tallerista_id.into(), nombre.into()],
		))
		.await?;
	let ahora = ahora(&txn).await?;
	if let Some(fila) = actual {
		let id: i32 = fila.try_get("", "id")?;
		let anterior: i32 = fila.try_get("", "cuota_mensual_mxn")?;
		if anterior != cuota {
			txn
				.execute_raw(Statement::from_sql_and_values(
					DbBackend::Postgres,
					"UPDATE talleres SET cuota_mensual_mxn = $1 WHERE id = $2",
					[cuota.into(), id.into()],
				))
				.await?;
			txn
				.execute_raw(Statement::from_sql_and_values(
					DbBackend::Postgres,
					"INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde)
                 VALUES ($1, $2, $3, $4)",
					[id.into(), anterior.into(), cuota.into(), ahora.into()],
				))
				.await?;
		}
	} else {
		txn
			.execute_raw(Statement::from_sql_and_values(
				DbBackend::Postgres,
				"INSERT INTO talleres (tallerista_id, nombre, cuota_mensual_mxn) VALUES ($1, $2, $3)",
				[tallerista_id.into(), nombre.into(), cuota.into()],
			))
			.await?;
		let id: i32 = txn
			.query_one_raw(Statement::from_sql_and_values(
				DbBackend::Postgres,
				"SELECT id FROM talleres WHERE tallerista_id = $1 AND nombre = $2",
				[tallerista_id.into(), nombre.into()],
			))
			.await?
			.ok_or_else(|| DbErr::Custom("no se pudo crear el taller".to_owned()))?
			.try_get("", "id")?;
		txn
			.execute_raw(Statement::from_sql_and_values(
				DbBackend::Postgres,
				"INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde)
             VALUES ($1, NULL, $2, $3)",
				[id.into(), cuota.into(), ahora.into()],
			))
			.await?;
	}
	txn.commit().await?;
	Ok(())
}

pub async fn guardar_horario_cuenta(
	db: &DatabaseConnection,
	tallerista_id: i32,
	taller: Option<&str>,
	dia: &str,
	inicio: &str,
	fin: &str,
) -> Result<(), DbErr> {
	let taller_id = buscar_taller(db, tallerista_id, taller).await?;
	let dia = dia_interno(dia)?;
	let inicio = hora(inicio)?;
	let fin = hora(fin)?;
	if inicio >= fin {
		return Err(aviso("la hora de inicio tiene que ser antes de la hora de fin"));
	}
	db.execute_raw(Statement::from_sql_and_values(
		DbBackend::Postgres,
		"INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
         SELECT $1, $2, $3, $4
         WHERE NOT EXISTS (
             SELECT 1 FROM horarios WHERE taller_id = $5 AND dia = $6 AND hora_inicio = $7
         )",
		[
			taller_id.into(),
			dia.into(),
			inicio.clone().into(),
			fin.into(),
			taller_id.into(),
			dia.into(),
			inicio.into(),
		],
	))
	.await?;
	Ok(())
}

pub(crate) async fn buscar_taller(
	db: &DatabaseConnection,
	tallerista_id: i32,
	nombre: Option<&str>,
) -> Result<i32, DbErr> {
	if let Some(nombre) = nombre.map(str::trim).filter(|nombre| !nombre.is_empty()) {
		return db
			.query_one_raw(Statement::from_sql_and_values(
				DbBackend::Postgres,
				"SELECT id FROM talleres WHERE tallerista_id = $1 AND nombre = $2 AND activo = 1",
				[tallerista_id.into(), nombre.into()],
			))
			.await?
			.ok_or_else(|| aviso("no encontré ese taller en tu cuenta"))?
			.try_get("", "id");
	}
	let filas = db
		.query_all_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id FROM talleres WHERE tallerista_id = $1 AND activo = 1 ORDER BY id",
			[tallerista_id.into()],
		))
		.await?;
	if filas.len() == 1 {
		filas[0].try_get("", "id")
	} else if filas.is_empty() {
		Err(aviso("primero crea el taller en /cuenta/taller"))
	} else {
		Err(aviso("indica el taller porque tienes más de uno"))
	}
}

pub(crate) fn dia_interno(dia: &str) -> Result<&'static str, DbErr> {
	match crate::middleware::auth::normalizar_usuario(dia).as_str() {
		"lunes" => Ok("lunes"),
		"martes" => Ok("martes"),
		"miercoles" => Ok("miercoles"),
		"jueves" => Ok("jueves"),
		"viernes" => Ok("viernes"),
		"sabado" => Ok("sabado"),
		"domingo" => Ok("domingo"),
		_ => Err(aviso(
			"el día debe ser lunes, martes, miércoles, jueves, viernes, sábado o domingo",
		)),
	}
}

pub(crate) fn hora(valor: &str) -> Result<String, DbErr> {
	let (horas, minutos) = valor
		.trim()
		.split_once(':')
		.ok_or_else(|| aviso("la hora debe verse como 16:00"))?;
	let horas: u32 = horas.parse().map_err(|_| aviso("la hora debe verse como 16:00"))?;
	let minutos: u32 = minutos.parse().map_err(|_| aviso("la hora debe verse como 16:00"))?;
	if horas > 23 || minutos > 59 {
		return Err(aviso("la hora debe verse como 16:00"));
	}
	Ok(format!("{horas:02}:{minutos:02}"))
}
