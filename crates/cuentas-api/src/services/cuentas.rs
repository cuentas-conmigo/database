//! Resumen de la cuenta del tallerista.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

use super::*;

#[derive(Debug, serde::Serialize)]
pub struct Cuenta {
	pub tallerista: String,
	pub usuario: String,
	pub telefono: String,
	pub oficio: String,
	pub listo: bool,
	pub falta: Vec<String>,
	pub siguientes: Siguientes,
	pub talleres: Vec<TallerCuenta>,
}

#[derive(Debug, serde::Serialize)]
pub struct Siguientes {
	pub datos: String,
	pub taller: String,
	pub horario: String,
	pub alumno: String,
	pub pago: String,
}

#[derive(Debug, serde::Serialize)]
pub struct TallerCuenta {
	pub nombre: String,
	pub cuota_mensual_mxn: i32,
	pub horarios: Vec<HorarioCuenta>,
	pub alumnos: Vec<AlumnoCuenta>,
}

#[derive(Debug, serde::Serialize)]
pub struct HorarioCuenta {
	pub dia: String,
	pub de: String,
	pub a: String,
}

#[derive(Debug, serde::Serialize)]
pub struct AlumnoCuenta {
	pub nombre: String,
	pub mensualidad: String,
	pub periodo: String,
}

pub async fn cuenta(db: &DatabaseConnection, sesion: &Sesion) -> Result<Cuenta, DbErr> {
	let fila = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT nombre, usuario, oficio, telefono FROM talleristas WHERE id = $1",
			[sesion.id.into()],
		))
		.await?
		.ok_or_else(|| DbErr::Custom("el tallerista no existe".to_owned()))?;
	let nombre: String = fila.try_get("", "nombre")?;
	let usuario: String = fila.try_get("", "usuario")?;
	let oficio: String = fila.try_get("", "oficio")?;
	let telefono: Option<String> = fila.try_get("", "telefono")?;

	let talleres = db
		.query_all_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id, nombre, cuota_mensual_mxn
             FROM talleres
             WHERE tallerista_id = $1 AND activo = 1
             ORDER BY id",
			[sesion.id.into()],
		))
		.await?;
	let horarios = db
		.query_all_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT h.taller_id, h.dia, h.hora_inicio, h.hora_fin
             FROM horarios h
             JOIN talleres t ON t.id = h.taller_id
             WHERE t.tallerista_id = $1
             ORDER BY h.taller_id,
                 CASE h.dia
                     WHEN 'lunes' THEN 1
                     WHEN 'martes' THEN 2
                     WHEN 'miercoles' THEN 3
                     WHEN 'jueves' THEN 4
                     WHEN 'viernes' THEN 5
                     WHEN 'sabado' THEN 6
                     ELSE 7
                 END,
                 h.hora_inicio",
			[sesion.id.into()],
		))
		.await?;
	let alumnos = db
		.query_all_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT
                 i.taller_id,
                 a.nombre,
                 COALESCE(
                     (
                         SELECT p.estado
                         FROM pagos p
                         WHERE p.alumno_id = a.id
                           AND p.taller_id = i.taller_id
                           AND p.tipo = 'mensualidad'
                           AND p.periodo = periodo_actual()
                         ORDER BY p.id DESC
                         LIMIT 1
                     ),
                     'pendiente'
                 ) AS mensualidad,
                 periodo_actual() AS periodo
             FROM inscripciones_taller i
             JOIN alumnos a ON a.id = i.alumno_id
             JOIN talleres t ON t.id = i.taller_id
             WHERE t.tallerista_id = $1
               AND i.activa = 1
               AND i.anio = anio_actual()
             ORDER BY i.taller_id, a.nombre",
			[sesion.id.into()],
		))
		.await?;

	let mut resultado = Vec::new();
	for taller in talleres {
		let taller_id: i32 = taller.try_get("", "id")?;
		let mut horarios_taller = Vec::new();
		for horario in &horarios {
			let id: i32 = horario.try_get("", "taller_id")?;
			if id != taller_id {
				continue;
			}
			let dia: String = horario.try_get("", "dia")?;
			horarios_taller.push(HorarioCuenta {
				dia: dia_visible(&dia).to_owned(),
				de: horario.try_get("", "hora_inicio")?,
				a: horario.try_get("", "hora_fin")?,
			});
		}
		let mut alumnos_taller = Vec::new();
		for alumno in &alumnos {
			let id: i32 = alumno.try_get("", "taller_id")?;
			if id != taller_id {
				continue;
			}
			alumnos_taller.push(AlumnoCuenta {
				nombre: alumno.try_get("", "nombre")?,
				mensualidad: alumno.try_get("", "mensualidad")?,
				periodo: alumno.try_get("", "periodo")?,
			});
		}
		resultado.push(TallerCuenta {
			nombre: taller.try_get("", "nombre")?,
			cuota_mensual_mxn: taller.try_get("", "cuota_mensual_mxn")?,
			horarios: horarios_taller,
			alumnos: alumnos_taller,
		});
	}

	let mut falta = Vec::new();
	if oficio.trim().is_empty() {
		falta.push("Escribe tu nombre y tu oficio en /cuenta/datos?nombre=&oficio=".to_string());
	}
	if resultado.is_empty() {
		falta.push("Crea el taller en /cuenta/taller?nombre=&cuota_mensual_mxn=25".to_string());
	} else if resultado.iter().any(|taller| taller.horarios.is_empty()) {
		falta.push("Agrega el horario en /cuenta/horario?dia=martes&de=16:00&a=18:00".to_string());
	}
	let taller_ejemplo = resultado
		.first()
		.map(|taller| url_parte(&taller.nombre))
		.unwrap_or_else(|| "Ceramica".to_string());

	Ok(Cuenta {
		tallerista: nombre,
		usuario,
		telefono: telefono.as_deref().map(enmascarar_telefono).unwrap_or_default(),
		oficio,
		listo: falta.is_empty(),
		falta,
		siguientes: Siguientes {
			datos: "/cuenta/datos?nombre=&oficio=".to_string(),
			taller: "/cuenta/taller?nombre=&cuota_mensual_mxn=25".to_string(),
			horario: format!("/cuenta/horario?taller={taller_ejemplo}&dia=martes&de=16:00&a=18:00"),
			alumno: format!("/cuenta/alumno?nombre=&taller={taller_ejemplo}"),
			pago: format!("/cuenta/pago?alumno=&taller={taller_ejemplo}&estado=pagado"),
		},
		talleres: resultado,
	})
}

pub(crate) fn enmascarar_telefono(telefono: &str) -> String {
	let visibles = telefono.chars().rev().take(4).collect::<String>();
	let visibles = visibles.chars().rev().collect::<String>();
	format!("******{visibles}")
}

pub async fn guardar_datos_cuenta(
	db: &DatabaseConnection,
	tallerista_id: i32,
	nombre: &str,
	oficio: &str,
) -> Result<(), DbErr> {
	let nombre = nombre.trim();
	let oficio = oficio.trim();
	if nombre.is_empty() || oficio.is_empty() {
		return Err(aviso("escribe nombre y oficio"));
	}
	db.execute_raw(Statement::from_sql_and_values(
		DbBackend::Postgres,
		"UPDATE talleristas SET nombre = $1, oficio = $2 WHERE id = $3",
		[nombre.into(), oficio.into(), tallerista_id.into()],
	))
	.await?;
	Ok(())
}
