use rusqlite::{Connection, OptionalExtension, params};

/// Inscripción anual por alumno, una vez por año.
pub const INSCRIPCION_ANUAL_MXN: i64 = 125;
/// Mensualidad inicial de cada taller. El tallerista puede cambiarla después.
pub const MENSUALIDAD_MXN: i64 = 25;

const SCHEMA: &str = "
PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS cuentas;

CREATE TABLE IF NOT EXISTS talleristas (
    id INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL,
    oficio TEXT NOT NULL,
    -- Id público de la cuenta Pey del tallerista (apiKeyId).
    -- No es la API key secreta y no es su CLABE.
    pey_api_key_id TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS talleres (
    id INTEGER PRIMARY KEY,
    tallerista_id INTEGER NOT NULL REFERENCES talleristas(id),
    nombre TEXT NOT NULL,
    cuota_mensual_mxn INTEGER NOT NULL CHECK (cuota_mensual_mxn > 0),
    activo INTEGER NOT NULL DEFAULT 1 CHECK (activo IN (0, 1)),
    UNIQUE (tallerista_id, nombre)
);

CREATE TABLE IF NOT EXISTS cambios_cuota (
    id INTEGER PRIMARY KEY,
    taller_id INTEGER NOT NULL REFERENCES talleres(id),
    cuota_anterior_mxn INTEGER,
    cuota_nueva_mxn INTEGER NOT NULL CHECK (cuota_nueva_mxn > 0),
    vigente_desde TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS alumnos (
    id INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL,
    cartera_ref TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS dispositivos_nfc (
    id INTEGER PRIMARY KEY,
    tallerista_id INTEGER NOT NULL REFERENCES talleristas(id),
    cara TEXT NOT NULL CHECK (cara IN ('cobro', 'inscripcion')),
    chip_uid TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS inscripciones_anuales (
    id INTEGER PRIMARY KEY,
    alumno_id INTEGER NOT NULL REFERENCES alumnos(id),
    anio INTEGER NOT NULL,
    monto_mxn INTEGER NOT NULL CHECK (monto_mxn > 0),
    UNIQUE (alumno_id, anio)
);

CREATE TABLE IF NOT EXISTS inscripciones_taller (
    id INTEGER PRIMARY KEY,
    alumno_id INTEGER NOT NULL REFERENCES alumnos(id),
    taller_id INTEGER NOT NULL REFERENCES talleres(id),
    anio INTEGER NOT NULL,
    activa INTEGER NOT NULL DEFAULT 1 CHECK (activa IN (0, 1)),
    UNIQUE (alumno_id, taller_id, anio)
);

CREATE TABLE IF NOT EXISTS pagos (
    id INTEGER PRIMARY KEY,
    alumno_id INTEGER NOT NULL REFERENCES alumnos(id),
    taller_id INTEGER REFERENCES talleres(id),
    tallerista_id INTEGER REFERENCES talleristas(id),
    tipo TEXT NOT NULL CHECK (tipo IN ('inscripcion_anual', 'mensualidad')),
    canal TEXT NOT NULL CHECK (canal IN ('boton', 'nfc', 'panel')),
    concepto TEXT NOT NULL,
    monto_mxn INTEGER NOT NULL CHECK (monto_mxn > 0),
    num_ref TEXT NOT NULL UNIQUE,
    codi_qr_id TEXT UNIQUE,
    transaction_id TEXT UNIQUE,
    tipo_evento TEXT CHECK (
        tipo_evento IN ('codi.qr.payment_received', 'codi.push.payment_received')
    ),
    estado TEXT NOT NULL CHECK (estado IN ('pendiente', 'pagado', 'fallido')),
    periodo TEXT NOT NULL,
    pagado_en TEXT,
    CHECK (
        (tipo = 'inscripcion_anual' AND taller_id IS NULL AND tallerista_id IS NULL)
        OR (tipo = 'mensualidad' AND taller_id IS NOT NULL AND tallerista_id IS NOT NULL)
    ),
    CHECK (
        (estado = 'pagado' AND transaction_id IS NOT NULL AND pagado_en IS NOT NULL)
        OR (estado <> 'pagado')
    )
);

CREATE INDEX IF NOT EXISTS idx_inscripciones_taller_taller
    ON inscripciones_taller (taller_id);
CREATE INDEX IF NOT EXISTS idx_pagos_tallerista
    ON pagos (tallerista_id, estado);
CREATE INDEX IF NOT EXISTS idx_pagos_alumno
    ON pagos (alumno_id);

DROP VIEW IF EXISTS ingresos_tallerista;
CREATE VIEW ingresos_tallerista AS
SELECT
    ta.id AS tallerista_id,
    ta.nombre,
    ta.oficio,
    COALESCE(SUM(p.monto_mxn), 0) AS total_cobrado_mxn
FROM talleristas ta
LEFT JOIN pagos p
    ON p.tallerista_id = ta.id
   AND p.estado = 'pagado'
GROUP BY ta.id;
";

const SEED: &str = "
INSERT INTO talleristas (id, nombre, oficio, pey_api_key_id) VALUES
    (1, 'Marichuy', 'tejido', 'ejemplo_pey_marichuy'),
    (2, 'Pepe', 'artes visuales', 'ejemplo_pey_pepe'),
    (3, 'María', 'matemáticas', 'ejemplo_pey_maria'),
    (4, 'Juan', 'deportes', 'ejemplo_pey_juan');

INSERT INTO talleres (id, tallerista_id, nombre, cuota_mensual_mxn) VALUES
    (1, 1, 'Tejido', 25),
    (2, 2, 'Artes visuales', 25),
    (3, 3, 'Matemáticas', 25),
    (4, 4, 'Deportes', 25);

INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde) VALUES
    (1, NULL, 25, '2026-01-01'),
    (2, NULL, 25, '2026-01-01'),
    (3, NULL, 25, '2026-01-01'),
    (4, NULL, 25, '2026-01-01');

INSERT INTO alumnos (id, nombre, cartera_ref) VALUES
    (1, 'Ana López', 'cartera_ana_lopez'),
    (2, 'Luis Hernández', 'cartera_luis_hernandez'),
    (3, 'Sofía Ramírez', 'cartera_sofia_ramirez'),
    (4, 'Diego Torres', 'cartera_diego_torres'),
    (5, 'Valeria Gómez', 'cartera_valeria_gomez'),
    (6, 'Mateo Cruz', 'cartera_mateo_cruz'),
    (7, 'Camila Flores', 'cartera_camila_flores'),
    (8, 'Javier Morales', 'cartera_javier_morales'),
    (9, 'Regina Ortiz', 'cartera_regina_ortiz'),
    (10, 'Emiliano Vargas', 'cartera_emiliano_vargas');

INSERT INTO dispositivos_nfc (tallerista_id, cara, chip_uid) VALUES
    (1, 'cobro', 'nfc-marichuy-cobro'),
    (1, 'inscripcion', 'nfc-marichuy-inscripcion'),
    (4, 'cobro', 'nfc-juan-cobro'),
    (4, 'inscripcion', 'nfc-juan-inscripcion');

INSERT INTO inscripciones_anuales (alumno_id, anio, monto_mxn) VALUES
    (1, 2026, 125),
    (2, 2026, 125),
    (3, 2026, 125),
    (4, 2026, 125),
    (5, 2026, 125),
    (6, 2026, 125),
    (7, 2026, 125),
    (8, 2026, 125),
    (9, 2026, 125),
    (10, 2026, 125);

INSERT INTO inscripciones_taller (alumno_id, taller_id, anio) VALUES
    (1, 1, 2026),
    (1, 3, 2026),
    (2, 2, 2026),
    (4, 1, 2026),
    (5, 3, 2026),
    (5, 4, 2026),
    (6, 2, 2026),
    (7, 1, 2026),
    (8, 4, 2026),
    (9, 3, 2026),
    (10, 2, 2026),
    (10, 1, 2026),
    (3, 4, 2026);

INSERT INTO pagos (
    alumno_id, taller_id, tallerista_id, tipo, canal, concepto, monto_mxn,
    num_ref, codi_qr_id, transaction_id, tipo_evento, estado, periodo, pagado_en
) VALUES
    (1, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-01', 'qr-anual-01', 'txn-anual-01', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-08 09:12:00'),
    (2, NULL, NULL, 'inscripcion_anual', 'boton', 'Inscripción anual 2026', 125,
        'ANUAL-2026-02', 'qr-anual-02', 'txn-anual-02', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-09 11:03:00'),
    (3, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-03', 'qr-anual-03', 'txn-anual-03', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-10 16:40:00'),
    (4, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-04', 'qr-anual-04', 'txn-anual-04', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-11 10:18:00'),
    (5, NULL, NULL, 'inscripcion_anual', 'boton', 'Inscripción anual 2026', 125,
        'ANUAL-2026-05', 'qr-anual-05', 'txn-anual-05', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-12 13:22:00'),
    (6, NULL, NULL, 'inscripcion_anual', 'panel', 'Inscripción anual 2026', 125,
        'ANUAL-2026-06', 'qr-anual-06', 'txn-anual-06', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-13 18:05:00'),
    (7, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-07', 'qr-anual-07', 'txn-anual-07', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-14 09:47:00'),
    (8, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-08', 'qr-anual-08', 'txn-anual-08', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-15 15:11:00'),
    (9, NULL, NULL, 'inscripcion_anual', 'panel', 'Inscripción anual 2026', 125,
        'ANUAL-2026-09', 'qr-anual-09', 'txn-anual-09', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-16 12:36:00'),
    (10, NULL, NULL, 'inscripcion_anual', 'boton', 'Inscripción anual 2026', 125,
        'ANUAL-2026-10', 'qr-anual-10', 'txn-anual-10', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-17 17:28:00'),
    (1, 1, 1, 'mensualidad', 'nfc', 'Mensualidad Tejido 2026-11', 25,
        'MENS-2026-11-01', 'qr-mens-01', 'txn-mens-01', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-02 08:10:00'),
    (1, 3, 3, 'mensualidad', 'panel', 'Mensualidad Matemáticas 2026-11', 25,
        'MENS-2026-11-02', 'qr-mens-02', 'txn-mens-02', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-02 08:16:00'),
    (2, 2, 2, 'mensualidad', 'panel', 'Mensualidad Artes visuales 2026-11', 25,
        'MENS-2026-11-03', 'qr-mens-03', 'txn-mens-03', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-03 19:02:00'),
    (3, 4, 4, 'mensualidad', 'nfc', 'Mensualidad Deportes 2026-11', 25,
        'MENS-2026-11-04', 'qr-mens-04', 'txn-mens-04', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-03 07:45:00'),
    (4, 1, 1, 'mensualidad', 'nfc', 'Mensualidad Tejido 2026-11', 25,
        'MENS-2026-11-05', 'qr-mens-05', 'txn-mens-05', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-04 18:20:00'),
    (5, 3, 3, 'mensualidad', 'panel', 'Mensualidad Matemáticas 2026-11', 25,
        'MENS-2026-11-06', 'qr-mens-06', 'txn-mens-06', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-04 20:11:00'),
    (5, 4, 4, 'mensualidad', 'nfc', 'Mensualidad Deportes 2026-11', 25,
        'MENS-2026-11-07', 'qr-mens-07', 'txn-mens-07', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-05 06:55:00'),
    (6, 2, 2, 'mensualidad', 'panel', 'Mensualidad Artes visuales 2026-11', 25,
        'MENS-2026-11-08', 'qr-mens-08', 'txn-mens-08', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-05 21:14:00'),
    (7, 1, 1, 'mensualidad', 'nfc', 'Mensualidad Tejido 2026-11', 25,
        'MENS-2026-11-09', 'qr-mens-09', NULL, NULL, 'pendiente', '2026-11', NULL),
    (8, 4, 4, 'mensualidad', 'nfc', 'Mensualidad Deportes 2026-11', 25,
        'MENS-2026-11-10', 'qr-mens-10', NULL, NULL, 'pendiente', '2026-11', NULL),
    (9, 3, 3, 'mensualidad', 'panel', 'Mensualidad Matemáticas 2026-11', 25,
        'MENS-2026-11-11', 'qr-mens-11', 'txn-mens-11', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-06 11:09:00'),
    (10, 2, 2, 'mensualidad', 'panel', 'Mensualidad Artes visuales 2026-11', 25,
        'MENS-2026-11-12', 'qr-mens-12', 'txn-mens-12', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-06 17:33:00'),
    (10, 1, 1, 'mensualidad', 'nfc', 'Mensualidad Tejido 2026-11', 25,
        'MENS-2026-11-13', 'qr-mens-13', 'txn-mens-13', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-07 09:27:00');
";

#[derive(Debug)]
pub struct Resumen {
    pub talleristas: i64,
    pub alumnos: i64,
    pub inscripciones_taller: i64,
    pub pagos_pagados: i64,
    pub pagos_pendientes: i64,
}

#[derive(Debug)]
pub struct IngresoTallerista {
    pub nombre: String,
    pub oficio: String,
    pub total_cobrado_mxn: i64,
}

/// Abre el archivo SQLite, crea las tablas y carga los datos de ejemplo
/// solo si todavía no hay talleristas.
pub fn open(path: &str) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    prepare(&mut conn)?;
    Ok(conn)
}

pub fn prepare(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA)?;
    seed(conn)?;
    Ok(())
}

pub fn seed(conn: &mut Connection) -> rusqlite::Result<()> {
    let existentes: i64 =
        conn.query_row("SELECT COUNT(*) FROM talleristas", [], |row| row.get(0))?;
    if existentes > 0 {
        return Ok(());
    }
    let tx = conn.transaction()?;
    tx.execute_batch(SEED)?;
    tx.commit()?;
    Ok(())
}

pub fn resumen(conn: &Connection) -> rusqlite::Result<Resumen> {
    Ok(Resumen {
        talleristas: count(conn, "SELECT COUNT(*) FROM talleristas")?,
        alumnos: count(conn, "SELECT COUNT(*) FROM alumnos")?,
        inscripciones_taller: count(conn, "SELECT COUNT(*) FROM inscripciones_taller")?,
        pagos_pagados: count(conn, "SELECT COUNT(*) FROM pagos WHERE estado = 'pagado'")?,
        pagos_pendientes: count(
            conn,
            "SELECT COUNT(*) FROM pagos WHERE estado = 'pendiente'",
        )?,
    })
}

pub fn ingresos(conn: &Connection) -> rusqlite::Result<Vec<IngresoTallerista>> {
    let mut stmt = conn.prepare(
        "SELECT nombre, oficio, total_cobrado_mxn
         FROM ingresos_tallerista
         ORDER BY tallerista_id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(IngresoTallerista {
            nombre: row.get(0)?,
            oficio: row.get(1)?,
            total_cobrado_mxn: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn listar(conn: &Connection, tabla: &str) -> Result<String, String> {
    let (encabezado, filas) = match tabla {
        "alumnos" => (
            "id | nombre | cartera",
            filas(
                conn,
                "SELECT id, nombre, cartera_ref FROM alumnos ORDER BY id",
                |row| {
                    Ok(format!(
                        "{} | {} | {}",
                        row.get::<_, i64>(0)?,
                        texto(row, 1)?,
                        texto(row, 2)?
                    ))
                },
            )?,
        ),
        "talleristas" => (
            "id | nombre | oficio | pey",
            filas(
                conn,
                "SELECT id, nombre, oficio, pey_api_key_id FROM talleristas ORDER BY id",
                |row| {
                    Ok(format!(
                        "{} | {} | {} | {}",
                        row.get::<_, i64>(0)?,
                        texto(row, 1)?,
                        texto(row, 2)?,
                        texto(row, 3)?
                    ))
                },
            )?,
        ),
        "talleres" => (
            "id | tallerista | taller | cuota | activo",
            filas(
                conn,
                "SELECT t.id, ta.nombre, t.nombre, t.cuota_mensual_mxn, t.activo
                 FROM talleres t
                 JOIN talleristas ta ON ta.id = t.tallerista_id
                 ORDER BY t.id",
                |row| {
                    Ok(format!(
                        "{} | {} | {} | {} | {}",
                        row.get::<_, i64>(0)?,
                        texto(row, 1)?,
                        texto(row, 2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?
                    ))
                },
            )?,
        ),
        "inscripciones" => (
            "id | alumno | taller | año | activa",
            filas(
                conn,
                "SELECT i.id, a.nombre, t.nombre, i.anio, i.activa
                 FROM inscripciones_taller i
                 JOIN alumnos a ON a.id = i.alumno_id
                 JOIN talleres t ON t.id = i.taller_id
                 ORDER BY i.id",
                |row| {
                    Ok(format!(
                        "{} | {} | {} | {} | {}",
                        row.get::<_, i64>(0)?,
                        texto(row, 1)?,
                        texto(row, 2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?
                    ))
                },
            )?,
        ),
        "anuales" => (
            "id | alumno | año | monto",
            filas(
                conn,
                "SELECT i.id, a.nombre, i.anio, i.monto_mxn
                 FROM inscripciones_anuales i
                 JOIN alumnos a ON a.id = i.alumno_id
                 ORDER BY i.id",
                |row| {
                    Ok(format!(
                        "{} | {} | {} | {}",
                        row.get::<_, i64>(0)?,
                        texto(row, 1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?
                    ))
                },
            )?,
        ),
        "pagos" => (
            "id | alumno | tipo | monto | estado | periodo | canal",
            filas(
                conn,
                "SELECT p.id, a.nombre, p.tipo, p.monto_mxn, p.estado, p.periodo, p.canal
                 FROM pagos p
                 JOIN alumnos a ON a.id = p.alumno_id
                 ORDER BY p.id",
                |row| {
                    Ok(format!(
                        "{} | {} | {} | {} | {} | {} | {}",
                        row.get::<_, i64>(0)?,
                        texto(row, 1)?,
                        texto(row, 2)?,
                        row.get::<_, i64>(3)?,
                        texto(row, 4)?,
                        texto(row, 5)?,
                        texto(row, 6)?
                    ))
                },
            )?,
        ),
        "nfc" => (
            "id | tallerista | cara | chip",
            filas(
                conn,
                "SELECT n.id, ta.nombre, n.cara, n.chip_uid
                 FROM dispositivos_nfc n
                 JOIN talleristas ta ON ta.id = n.tallerista_id
                 ORDER BY n.id",
                |row| {
                    Ok(format!(
                        "{} | {} | {} | {}",
                        row.get::<_, i64>(0)?,
                        texto(row, 1)?,
                        texto(row, 2)?,
                        texto(row, 3)?
                    ))
                },
            )?,
        ),
        "cuotas" => (
            "id | taller | anterior | nueva | desde",
            filas(
                conn,
                "SELECT c.id, t.nombre, c.cuota_anterior_mxn, c.cuota_nueva_mxn, c.vigente_desde
                 FROM cambios_cuota c
                 JOIN talleres t ON t.id = c.taller_id
                 ORDER BY c.id",
                |row| {
                    let anterior: Option<i64> = row.get(2)?;
                    Ok(format!(
                        "{} | {} | {} | {} | {}",
                        row.get::<_, i64>(0)?,
                        texto(row, 1)?,
                        anterior
                            .map(|monto| monto.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        row.get::<_, i64>(3)?,
                        texto(row, 4)?
                    ))
                },
            )?,
        ),
        "ingresos" => (
            "tallerista | oficio | cobrado",
            ingresos(conn)
                .map_err(explicar)?
                .into_iter()
                .map(|ingreso| {
                    format!(
                        "{} | {} | {}",
                        ingreso.nombre, ingreso.oficio, ingreso.total_cobrado_mxn
                    )
                })
                .collect(),
        ),
        _ => return Err(format!("no se puede listar '{tabla}'")),
    };
    Ok(unir(encabezado, filas))
}

pub fn agregar_alumno(conn: &Connection, nombre: &str, cartera: &str) -> Result<i64, String> {
    exigir_texto(nombre, "nombre")?;
    exigir_texto(cartera, "cartera")?;
    conn.execute(
        "INSERT INTO alumnos (nombre, cartera_ref) VALUES (?1, ?2)",
        params![nombre.trim(), cartera.trim()],
    )
    .map_err(explicar)?;
    Ok(conn.last_insert_rowid())
}

pub fn cambiar_alumno(
    conn: &Connection,
    id: i64,
    nombre: Option<&str>,
    cartera: Option<&str>,
) -> Result<bool, String> {
    let actual = uno(
        conn,
        "SELECT nombre, cartera_ref FROM alumnos WHERE id = ?1",
        [id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;
    let Some((nombre_actual, cartera_actual)) = actual else {
        return Ok(false);
    };
    let nombre = nombre.unwrap_or(&nombre_actual);
    let cartera = cartera.unwrap_or(&cartera_actual);
    exigir_texto(nombre, "nombre")?;
    exigir_texto(cartera, "cartera")?;
    let cambiados = conn
        .execute(
            "UPDATE alumnos SET nombre = ?1, cartera_ref = ?2 WHERE id = ?3",
            params![nombre.trim(), cartera.trim(), id],
        )
        .map_err(explicar)?;
    Ok(cambiados > 0)
}

pub fn agregar_tallerista(
    conn: &Connection,
    nombre: &str,
    oficio: &str,
    pey: &str,
) -> Result<i64, String> {
    exigir_texto(nombre, "nombre")?;
    exigir_texto(oficio, "oficio")?;
    exigir_texto(pey, "pey")?;
    conn.execute(
        "INSERT INTO talleristas (nombre, oficio, pey_api_key_id) VALUES (?1, ?2, ?3)",
        params![nombre.trim(), oficio.trim(), pey.trim()],
    )
    .map_err(explicar)?;
    Ok(conn.last_insert_rowid())
}

pub fn cambiar_tallerista(
    conn: &Connection,
    id: i64,
    nombre: Option<&str>,
    oficio: Option<&str>,
    pey: Option<&str>,
) -> Result<bool, String> {
    let actual = uno(
        conn,
        "SELECT nombre, oficio, pey_api_key_id FROM talleristas WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
    )?;
    let Some((nombre_actual, oficio_actual, pey_actual)) = actual else {
        return Ok(false);
    };
    let nombre = nombre.unwrap_or(&nombre_actual);
    let oficio = oficio.unwrap_or(&oficio_actual);
    let pey = pey.unwrap_or(&pey_actual);
    exigir_texto(nombre, "nombre")?;
    exigir_texto(oficio, "oficio")?;
    exigir_texto(pey, "pey")?;
    let cambiados = conn
        .execute(
            "UPDATE talleristas SET nombre = ?1, oficio = ?2, pey_api_key_id = ?3 WHERE id = ?4",
            params![nombre.trim(), oficio.trim(), pey.trim(), id],
        )
        .map_err(explicar)?;
    Ok(cambiados > 0)
}

pub fn agregar_taller(
    conn: &mut Connection,
    tallerista_id: i64,
    nombre: &str,
    cuota: i64,
) -> Result<i64, String> {
    exigir_texto(nombre, "nombre")?;
    exigir_monto(cuota)?;
    let tx = conn.transaction().map_err(explicar)?;
    tx.execute(
        "INSERT INTO talleres (tallerista_id, nombre, cuota_mensual_mxn) VALUES (?1, ?2, ?3)",
        params![tallerista_id, nombre.trim(), cuota],
    )
    .map_err(explicar)?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde)
         VALUES (?1, NULL, ?2, datetime('now'))",
        params![id, cuota],
    )
    .map_err(explicar)?;
    tx.commit().map_err(explicar)?;
    Ok(id)
}

/// Cambia la mensualidad y guarda la cuota anterior.
/// `Ok(None)` significa que ese taller no existe.
pub fn cambiar_cuota(
    conn: &mut Connection,
    taller_id: i64,
    nueva: i64,
) -> Result<Option<(i64, i64)>, String> {
    exigir_monto(nueva)?;
    let tx = conn.transaction().map_err(explicar)?;
    let anterior: Option<i64> = tx
        .query_row(
            "SELECT cuota_mensual_mxn FROM talleres WHERE id = ?1",
            [taller_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(explicar)?;
    let Some(anterior) = anterior else {
        return Ok(None);
    };
    if anterior != nueva {
        tx.execute(
            "UPDATE talleres SET cuota_mensual_mxn = ?1 WHERE id = ?2",
            params![nueva, taller_id],
        )
        .map_err(explicar)?;
        tx.execute(
            "INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde)
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![taller_id, anterior, nueva],
        )
        .map_err(explicar)?;
    }
    tx.commit().map_err(explicar)?;
    Ok(Some((anterior, nueva)))
}

pub fn agregar_nfc(
    conn: &Connection,
    tallerista_id: i64,
    cara: &str,
    chip: &str,
) -> Result<i64, String> {
    exigir_cara(cara)?;
    exigir_texto(chip, "chip")?;
    conn.execute(
        "INSERT INTO dispositivos_nfc (tallerista_id, cara, chip_uid) VALUES (?1, ?2, ?3)",
        params![tallerista_id, cara, chip.trim()],
    )
    .map_err(explicar)?;
    Ok(conn.last_insert_rowid())
}

pub fn agregar_anual(
    conn: &Connection,
    alumno_id: i64,
    anio: Option<i64>,
    monto: Option<i64>,
) -> Result<i64, String> {
    let monto = monto.unwrap_or(INSCRIPCION_ANUAL_MXN);
    exigir_monto(monto)?;
    conn.execute(
        "INSERT INTO inscripciones_anuales (alumno_id, anio, monto_mxn)
         VALUES (?1, COALESCE(?2, CAST(strftime('%Y', 'now') AS INTEGER)), ?3)",
        params![alumno_id, anio, monto],
    )
    .map_err(explicar)?;
    Ok(conn.last_insert_rowid())
}

pub fn agregar_inscripcion(
    conn: &Connection,
    alumno_id: i64,
    taller_id: i64,
    anio: Option<i64>,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO inscripciones_taller (alumno_id, taller_id, anio)
         VALUES (?1, ?2, COALESCE(?3, CAST(strftime('%Y', 'now') AS INTEGER)))",
        params![alumno_id, taller_id, anio],
    )
    .map_err(explicar)?;
    Ok(conn.last_insert_rowid())
}

pub fn cambiar_inscripcion(conn: &Connection, id: i64, activa: i64) -> Result<bool, String> {
    if activa != 0 && activa != 1 {
        return Err("activa debe ser 0 o 1".to_string());
    }
    let cambiados = conn
        .execute(
            "UPDATE inscripciones_taller SET activa = ?1 WHERE id = ?2",
            params![activa, id],
        )
        .map_err(explicar)?;
    Ok(cambiados > 0)
}

pub struct NuevoPago<'a> {
    pub alumno_id: i64,
    pub tipo: &'a str,
    pub canal: &'a str,
    pub periodo: &'a str,
    pub taller_id: Option<i64>,
    pub monto: Option<i64>,
    pub num_ref: Option<&'a str>,
    pub estado: &'a str,
}

pub fn agregar_pago(conn: &Connection, pago: NuevoPago<'_>) -> Result<i64, String> {
    exigir_tipo(pago.tipo)?;
    exigir_canal(pago.canal)?;
    exigir_estado(pago.estado)?;
    exigir_texto(pago.periodo, "periodo")?;

    let (taller_id, tallerista_id, concepto, cuota) = if pago.tipo == "mensualidad" {
        let taller_id = pago.taller_id.ok_or("la mensualidad necesita --taller")?;
        let (tallerista_id, nombre, cuota): (i64, String, i64) = conn
            .query_row(
                "SELECT tallerista_id, nombre, cuota_mensual_mxn FROM talleres WHERE id = ?1",
                [taller_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| {
                if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                    "No hay un taller con ese id.".to_string()
                } else {
                    explicar(error)
                }
            })?;
        (
            Some(taller_id),
            Some(tallerista_id),
            format!("Mensualidad {nombre} {}", pago.periodo.trim()),
            cuota,
        )
    } else if pago.taller_id.is_some() {
        return Err("la inscripción anual no lleva --taller".to_string());
    } else {
        (
            None,
            None,
            format!("Inscripción anual {}", pago.periodo.trim()),
            INSCRIPCION_ANUAL_MXN,
        )
    };

    let monto = pago.monto.unwrap_or(cuota);
    exigir_monto(monto)?;
    let num_ref = pago
        .num_ref
        .map(str::trim)
        .filter(|valor| !valor.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "manual-{}-{}-{}-{}",
                pago.tipo,
                pago.alumno_id,
                pago.periodo.trim(),
                marca_ms()
            )
        });
    let qr = format!("qr-{num_ref}");
    let (transaction_id, tipo_evento, pagado_en) = if pago.estado == "pagado" {
        (
            Some(format!("txn-{num_ref}")),
            Some("codi.qr.payment_received".to_string()),
            Some(ahora(conn)?),
        )
    } else {
        (None, None, None)
    };

    conn.execute(
        "INSERT INTO pagos (
            alumno_id, taller_id, tallerista_id, tipo, canal, concepto, monto_mxn,
            num_ref, codi_qr_id, transaction_id, tipo_evento, estado, periodo, pagado_en
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            pago.alumno_id,
            taller_id,
            tallerista_id,
            pago.tipo,
            pago.canal,
            concepto,
            monto,
            num_ref,
            qr,
            transaction_id,
            tipo_evento,
            pago.estado,
            pago.periodo.trim(),
            pagado_en,
        ],
    )
    .map_err(explicar)?;
    Ok(conn.last_insert_rowid())
}

pub fn cambiar_pago(
    conn: &Connection,
    id: i64,
    estado: Option<&str>,
    monto: Option<i64>,
) -> Result<bool, String> {
    if let Some(estado) = estado {
        exigir_estado(estado)?;
    }
    if let Some(monto) = monto {
        exigir_monto(monto)?;
    }
    let actual = uno(
        conn,
        "SELECT monto_mxn, estado, transaction_id, tipo_evento, pagado_en
         FROM pagos WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        },
    )?;
    let Some((monto_actual, estado_actual, mut transaction_id, mut tipo_evento, mut pagado_en)) =
        actual
    else {
        return Ok(false);
    };
    let monto = monto.unwrap_or(monto_actual);
    let estado = estado.unwrap_or(&estado_actual).to_string();
    if estado == "pagado" {
        if transaction_id.is_none() {
            transaction_id = Some(format!("txn-manual-{id}"));
        }
        if tipo_evento.is_none() {
            tipo_evento = Some("codi.qr.payment_received".to_string());
        }
        if pagado_en.is_none() {
            pagado_en = Some(ahora(conn)?);
        }
    } else {
        pagado_en = None;
        tipo_evento = None;
    }
    let cambiados = conn
        .execute(
            "UPDATE pagos
             SET monto_mxn = ?1, estado = ?2, transaction_id = ?3, tipo_evento = ?4, pagado_en = ?5
             WHERE id = ?6",
            params![monto, estado, transaction_id, tipo_evento, pagado_en, id],
        )
        .map_err(explicar)?;
    Ok(cambiados > 0)
}

pub fn borrar(conn: &Connection, tabla: &str, id: i64) -> Result<bool, String> {
    let tabla = match tabla {
        "alumnos"
        | "talleristas"
        | "talleres"
        | "dispositivos_nfc"
        | "inscripciones_anuales"
        | "inscripciones_taller"
        | "pagos" => tabla,
        _ => return Err(format!("no se puede borrar en '{tabla}'")),
    };
    let cambiados = conn
        .execute(&format!("DELETE FROM {tabla} WHERE id = ?1"), [id])
        .map_err(explicar)?;
    Ok(cambiados > 0)
}

fn unir(encabezado: &str, filas: Vec<String>) -> String {
    if filas.is_empty() {
        format!("{encabezado}\n(sin registros)")
    } else {
        format!("{encabezado}\n{}", filas.join("\n"))
    }
}

fn filas<T, F>(conn: &Connection, sql: &str, mut mapa: F) -> Result<Vec<T>, String>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut stmt = conn.prepare(sql).map_err(explicar)?;
    let rows = stmt.query_map([], |row| mapa(row)).map_err(explicar)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(explicar)
}

fn uno<T, P, F>(conn: &Connection, sql: &str, params: P, mapa: F) -> Result<Option<T>, String>
where
    P: rusqlite::Params,
    F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    conn.query_row(sql, params, mapa)
        .optional()
        .map_err(explicar)
}

fn texto(row: &rusqlite::Row<'_>, indice: usize) -> rusqlite::Result<String> {
    row.get(indice)
}

fn exigir_texto(valor: &str, campo: &str) -> Result<(), String> {
    if valor.trim().is_empty() {
        Err(format!("{campo} no puede estar vacío"))
    } else {
        Ok(())
    }
}

fn exigir_monto(monto: i64) -> Result<(), String> {
    if monto <= 0 {
        Err("el monto debe ser mayor que 0".to_string())
    } else {
        Ok(())
    }
}

fn exigir_cara(cara: &str) -> Result<(), String> {
    if cara == "cobro" || cara == "inscripcion" {
        Ok(())
    } else {
        Err("la cara debe ser cobro o inscripcion".to_string())
    }
}

fn exigir_tipo(tipo: &str) -> Result<(), String> {
    if tipo == "mensualidad" || tipo == "inscripcion_anual" {
        Ok(())
    } else {
        Err("el tipo debe ser mensualidad o inscripcion_anual".to_string())
    }
}

fn exigir_canal(canal: &str) -> Result<(), String> {
    if canal == "boton" || canal == "nfc" || canal == "panel" {
        Ok(())
    } else {
        Err("el canal debe ser boton, nfc o panel".to_string())
    }
}

fn exigir_estado(estado: &str) -> Result<(), String> {
    if estado == "pendiente" || estado == "pagado" || estado == "fallido" {
        Ok(())
    } else {
        Err("el estado debe ser pendiente, pagado o fallido".to_string())
    }
}

fn ahora(conn: &Connection) -> Result<String, String> {
    conn.query_row("SELECT datetime('now')", [], |row| row.get(0))
        .map_err(explicar)
}

fn marca_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duracion| duracion.as_millis())
        .unwrap_or(0)
}

fn explicar(error: rusqlite::Error) -> String {
    let texto = error.to_string();
    if texto.contains("FOREIGN KEY") {
        "Ese id no existe, o el registro todavía tiene inscripciones, pagos o talleres ligados."
            .to_string()
    } else if texto.contains("UNIQUE") {
        "Ya existe un registro con ese dato.".to_string()
    } else if texto.contains("CHECK") {
        "Algún dato no cumple las reglas de monto, canal, estado o tipo.".to_string()
    } else {
        texto
    }
}

fn count(conn: &Connection, sql: &str) -> rusqlite::Result<i64> {
    conn.query_row(sql, [], |row| row.get(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        prepare(&mut conn).unwrap();
        conn
    }

    #[test]
    fn carga_el_ejemplo_una_sola_vez() {
        let mut conn = memory_db();
        let resumen = resumen(&conn).unwrap();

        assert_eq!(resumen.talleristas, 4);
        assert_eq!(resumen.alumnos, 10);
        assert_eq!(resumen.inscripciones_taller, 13);
        assert_eq!(resumen.pagos_pendientes, 2);

        seed(&mut conn).unwrap();
        assert_eq!(super::resumen(&conn).unwrap().alumnos, 10);
    }

    #[test]
    fn las_cuotas_de_ejemplo_son_125_y_25() {
        let conn = memory_db();
        let anual: i64 = conn
            .query_row(
                "SELECT DISTINCT monto_mxn FROM inscripciones_anuales",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mensual: i64 = conn
            .query_row(
                "SELECT DISTINCT cuota_mensual_mxn FROM talleres",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(anual, INSCRIPCION_ANUAL_MXN);
        assert_eq!(mensual, MENSUALIDAD_MXN);
    }

    #[test]
    fn el_ingreso_de_cada_tallerista_suma_solo_mensualidades_pagadas() {
        let conn = memory_db();
        let ingresos = ingresos(&conn).unwrap();

        let montos: Vec<(&str, i64)> = ingresos
            .iter()
            .map(|ingreso| (ingreso.nombre.as_str(), ingreso.total_cobrado_mxn))
            .collect();

        assert_eq!(
            montos,
            vec![("Marichuy", 75), ("Pepe", 75), ("María", 75), ("Juan", 50),]
        );
        let pendientes = listar(&conn, "pagos").unwrap();
        assert!(pendientes.contains("Camila Flores | mensualidad | 25 | pendiente"));
        assert!(pendientes.contains("Javier Morales | mensualidad | 25 | pendiente"));
    }

    #[test]
    fn rechaza_una_inscripcion_sin_alumno() {
        let conn = memory_db();
        let resultado = conn.execute(
            "INSERT INTO inscripciones_anuales (alumno_id, anio, monto_mxn) VALUES (999, 2026, 125)",
            [],
        );
        assert!(resultado.is_err());
    }
}
