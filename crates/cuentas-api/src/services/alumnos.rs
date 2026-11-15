//! Alumnos de la cuenta.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait};

use super::*;

pub async fn guardar_alumno_cuenta(
	db: &DatabaseConnection,
	tallerista_id: i32,
	nombre: &str,
	taller: Option<&str>,
) -> Result<(), DbErr> {
	let nombre = nombre.trim();
	if nombre.is_empty() {
		return Err(aviso("escribe el nombre del alumno"));
	}
	let taller_id = buscar_taller(db, tallerista_id, taller).await?;
	let txn = db.begin().await?;
	let alumno_id: i32 = match txn
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id FROM alumnos WHERE nombre = $1",
			[nombre.into()],
		))
		.await?
	{
		Some(fila) => fila.try_get::<i32>("", "id")?,
		None => {
			let cartera = format!(
				"cartera_{}_{tallerista_id}",
				crate::middleware::auth::normalizar_usuario(nombre)
			);
			txn
				.execute_raw(Statement::from_sql_and_values(
					DbBackend::Postgres,
					"INSERT INTO alumnos (nombre, cartera_ref) VALUES ($1, $2)",
					[nombre.into(), cartera.into()],
				))
				.await?;
			txn
				.query_one_raw(Statement::from_sql_and_values(
					DbBackend::Postgres,
					"SELECT id FROM alumnos WHERE nombre = $1",
					[nombre.into()],
				))
				.await?
				.ok_or_else(|| DbErr::Custom("no se pudo guardar al alumno".to_owned()))?
				.try_get::<i32>("", "id")?
		}
	};
	txn
		.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO inscripciones_taller (alumno_id, taller_id, anio)
         SELECT $1, $2, anio_actual()
         WHERE NOT EXISTS (
             SELECT 1 FROM inscripciones_taller
             WHERE alumno_id = $3 AND taller_id = $4 AND anio = anio_actual()
         )",
			[alumno_id.into(), taller_id.into(), alumno_id.into(), taller_id.into()],
		))
		.await?;
	txn
		.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO inscripciones_anuales (alumno_id, anio, monto_mxn)
         SELECT $1, anio_actual(), $2
         WHERE NOT EXISTS (
             SELECT 1 FROM inscripciones_anuales
             WHERE alumno_id = $3 AND anio = anio_actual()
         )",
			[alumno_id.into(), INSCRIPCION_ANUAL_MXN.into(), alumno_id.into()],
		))
		.await?;
	let num_ref = format!("alta-{tallerista_id}-{alumno_id}-{}", marca());
	txn
		.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO pagos (
             alumno_id, taller_id, tallerista_id, tipo, canal, concepto, monto_mxn,
             num_ref, estado, periodo
         )
         SELECT $1, $2, $3, 'mensualidad', 'panel',
                'Mensualidad ' || (SELECT nombre FROM talleres WHERE id = $4) || ' ' || periodo_actual(),
                (SELECT cuota_mensual_mxn FROM talleres WHERE id = $5),
                $6, 'pendiente', periodo_actual()
         WHERE NOT EXISTS (
             SELECT 1 FROM pagos
             WHERE alumno_id = $7 AND taller_id = $8 AND tipo = 'mensualidad'
               AND periodo = periodo_actual()
         )",
			[
				alumno_id.into(),
				taller_id.into(),
				tallerista_id.into(),
				taller_id.into(),
				taller_id.into(),
				num_ref.into(),
				alumno_id.into(),
				taller_id.into(),
			],
		))
		.await?;
	txn.commit().await?;
	Ok(())
}
