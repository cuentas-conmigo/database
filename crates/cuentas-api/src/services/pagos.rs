//! Pagos de la cuenta.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

use super::*;

pub async fn marcar_pago_cuenta(
	db: &DatabaseConnection,
	tallerista_id: i32,
	alumno: &str,
	taller: Option<&str>,
	estado: &str,
) -> Result<(), DbErr> {
	if estado != "pagado" && estado != "pendiente" && estado != "fallido" {
		return Err(aviso("el estado debe ser pagado, pendiente o fallido"));
	}
	let nombre = alumno.trim();
	if nombre.is_empty() {
		return Err(aviso("escribe el nombre del alumno"));
	}
	let taller_id = buscar_taller(db, tallerista_id, taller).await?;
	let alumno_id: i32 = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT a.id
             FROM alumnos a
             JOIN inscripciones_taller i ON i.alumno_id = a.id
             WHERE a.nombre = $1 AND i.taller_id = $2 AND i.activa = 1",
			[nombre.into(), taller_id.into()],
		))
		.await?
		.ok_or_else(|| aviso("ese alumno no está en tu taller"))?
		.try_get("", "id")?;
	let pago = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id FROM pagos
             WHERE alumno_id = $1 AND taller_id = $2 AND tipo = 'mensualidad'
               AND periodo = periodo_actual()
             ORDER BY id DESC
             LIMIT 1",
			[alumno_id.into(), taller_id.into()],
		))
		.await?;
	let ahora = ahora(db).await?;
	if let Some(fila) = pago {
		let id: i32 = fila.try_get("", "id")?;
		if estado == "pagado" {
			db.execute_raw(Statement::from_sql_and_values(
				DbBackend::Postgres,
				"UPDATE pagos
                 SET estado = 'pagado',
                     transaction_id = COALESCE(transaction_id, $1),
                     tipo_evento = COALESCE(tipo_evento, 'codi.qr.payment_received'),
                     pagado_en = COALESCE(pagado_en, $2)
                 WHERE id = $3",
				[format!("txn-cuenta-{id}").into(), ahora.into(), id.into()],
			))
			.await?;
		} else {
			db.execute_raw(Statement::from_sql_and_values(
				DbBackend::Postgres,
				"UPDATE pagos SET estado = $1 WHERE id = $2",
				[estado.into(), id.into()],
			))
			.await?;
		}
	} else if estado == "pagado" {
		let num_ref = format!("pago-{tallerista_id}-{alumno_id}-{}", marca());
		db.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO pagos (
                 alumno_id, taller_id, tallerista_id, tipo, canal, concepto, monto_mxn,
                 num_ref, transaction_id, tipo_evento, estado, periodo, pagado_en
             )
             SELECT $1, $2, $3, 'mensualidad', 'panel',
                    'Mensualidad ' || (SELECT nombre FROM talleres WHERE id = $4) || ' ' || periodo_actual(),
                    (SELECT cuota_mensual_mxn FROM talleres WHERE id = $5),
                    $6, $7, 'codi.qr.payment_received', 'pagado', periodo_actual(), $8",
			[
				alumno_id.into(),
				taller_id.into(),
				tallerista_id.into(),
				taller_id.into(),
				taller_id.into(),
				num_ref.clone().into(),
				format!("txn-{num_ref}").into(),
				ahora.into(),
			],
		))
		.await?;
	} else {
		return Err(aviso("todavía no hay una mensualidad de este mes para ese alumno"));
	}
	Ok(())
}
