use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::db::{self, NuevoPago};

const AYUDA: &str = "\
Uso: cargo run -- <comando> [opciones]

resumen
listar alumnos | talleristas | talleres | inscripciones | anuales | pagos | nfc | cuotas | ingresos

agregar alumno --nombre NOMBRE --cartera REF
agregar tallerista --nombre NOMBRE --oficio OFICIO --pey REF
agregar taller --tallerista ID --nombre NOMBRE [--cuota 25]
agregar nfc --tallerista ID --cara cobro|inscripcion --chip UID
agregar anual --alumno ID [--anio 2026] [--monto 125]
agregar inscripcion --alumno ID --taller ID [--anio 2026]
agregar pago --alumno ID --tipo mensualidad|inscripcion_anual --canal boton|nfc|panel --periodo 2026-11 [--taller ID] [--monto N] [--num-ref REF] [--estado pendiente]

cambiar alumno --id ID [--nombre NOMBRE] [--cartera REF]
cambiar tallerista --id ID [--nombre NOMBRE] [--oficio OFICIO] [--pey REF]
cambiar cuota --taller ID --monto N
cambiar inscripcion --id ID --activa 0|1
cambiar pago --id ID [--estado pendiente|pagado|fallido] [--monto N]

borrar alumnos | talleristas | talleres | nfc | anuales | inscripciones | pagos --id ID
";

pub fn ejecutar(conn: &mut Connection, args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0] == "ayuda" {
        return Ok(AYUDA.to_string());
    }
    match args[0].as_str() {
        "resumen" => resumen(conn),
        "listar" => {
            let tabla = args
                .get(1)
                .ok_or("indica qué listar, por ejemplo: listar alumnos")?;
            db::listar(conn, tabla)
        }
        "agregar" => agregar(
            conn,
            args.get(1).map(String::as_str),
            &banderas(&args[2..])?,
        ),
        "cambiar" => cambiar(
            conn,
            args.get(1).map(String::as_str),
            &banderas(&args[2..])?,
        ),
        "borrar" => borrar(
            conn,
            args.get(1).map(String::as_str),
            &banderas(&args[2..])?,
        ),
        otro => Err(format!("comando desconocido: {otro}\n\n{AYUDA}")),
    }
}

fn resumen(conn: &Connection) -> Result<String, String> {
    let datos = db::resumen(conn).map_err(|error| error.to_string())?;
    let mut texto = format!(
        "Inscripción anual: ${} MXN. Mensualidad inicial: ${} MXN.\n{} talleristas, {} alumnos, {} inscripciones a talleres\n{} pagos pagados y {} pendientes",
        db::INSCRIPCION_ANUAL_MXN,
        db::MENSUALIDAD_MXN,
        datos.talleristas,
        datos.alumnos,
        datos.inscripciones_taller,
        datos.pagos_pagados,
        datos.pagos_pendientes
    );
    for ingreso in db::ingresos(conn).map_err(|error| error.to_string())? {
        texto.push_str(&format!(
            "\n{} ({}): ${} MXN cobrados",
            ingreso.nombre, ingreso.oficio, ingreso.total_cobrado_mxn
        ));
    }
    Ok(texto)
}

fn agregar(
    conn: &mut Connection,
    que: Option<&str>,
    flags: &BTreeMap<String, String>,
) -> Result<String, String> {
    match que.ok_or("indica qué agregar, por ejemplo: agregar alumno")? {
        "alumno" => {
            let id = db::agregar_alumno(
                conn,
                &obligatorio(flags, "nombre")?,
                &obligatorio(flags, "cartera")?,
            )?;
            Ok(format!("Alumno {id} creado"))
        }
        "tallerista" => {
            let id = db::agregar_tallerista(
                conn,
                &obligatorio(flags, "nombre")?,
                &obligatorio(flags, "oficio")?,
                &obligatorio(flags, "pey")?,
            )?;
            Ok(format!("Tallerista {id} creado"))
        }
        "taller" => {
            let id = db::agregar_taller(
                conn,
                numero(flags, "tallerista")?,
                &obligatorio(flags, "nombre")?,
                numero_opcional(flags, "cuota")?.unwrap_or(db::MENSUALIDAD_MXN),
            )?;
            Ok(format!("Taller {id} creado"))
        }
        "nfc" => {
            let id = db::agregar_nfc(
                conn,
                numero(flags, "tallerista")?,
                &obligatorio(flags, "cara")?,
                &obligatorio(flags, "chip")?,
            )?;
            Ok(format!("Chip NFC {id} creado"))
        }
        "anual" => {
            let id = db::agregar_anual(
                conn,
                numero(flags, "alumno")?,
                numero_opcional(flags, "anio")?,
                numero_opcional(flags, "monto")?,
            )?;
            Ok(format!("Inscripción anual {id} creada"))
        }
        "inscripcion" => {
            let id = db::agregar_inscripcion(
                conn,
                numero(flags, "alumno")?,
                numero(flags, "taller")?,
                numero_opcional(flags, "anio")?,
            )?;
            Ok(format!("Inscripción {id} creada"))
        }
        "pago" => {
            let tipo = obligatorio(flags, "tipo")?;
            let canal = obligatorio(flags, "canal")?;
            let periodo = obligatorio(flags, "periodo")?;
            let id = db::agregar_pago(
                conn,
                NuevoPago {
                    alumno_id: numero(flags, "alumno")?,
                    tipo: &tipo,
                    canal: &canal,
                    periodo: &periodo,
                    taller_id: numero_opcional(flags, "taller")?,
                    monto: numero_opcional(flags, "monto")?,
                    num_ref: flags.get("num-ref").map(String::as_str),
                    estado: flags
                        .get("estado")
                        .map(String::as_str)
                        .unwrap_or("pendiente"),
                },
            )?;
            Ok(format!("Pago {id} creado"))
        }
        otro => Err(format!("no se puede agregar '{otro}'")),
    }
}

fn cambiar(
    conn: &mut Connection,
    que: Option<&str>,
    flags: &BTreeMap<String, String>,
) -> Result<String, String> {
    match que.ok_or("indica qué cambiar")? {
        "alumno" => {
            let id = numero(flags, "id")?;
            if !flags.contains_key("nombre") && !flags.contains_key("cartera") {
                return Err("indica --nombre o --cartera".to_string());
            }
            if db::cambiar_alumno(
                conn,
                id,
                flags.get("nombre").map(String::as_str),
                flags.get("cartera").map(String::as_str),
            )? {
                Ok(format!("Alumno {id} actualizado"))
            } else {
                Ok(format!("No hay alumno con id {id}"))
            }
        }
        "tallerista" => {
            let id = numero(flags, "id")?;
            if !flags.contains_key("nombre")
                && !flags.contains_key("oficio")
                && !flags.contains_key("pey")
            {
                return Err("indica --nombre, --oficio o --pey".to_string());
            }
            if db::cambiar_tallerista(
                conn,
                id,
                flags.get("nombre").map(String::as_str),
                flags.get("oficio").map(String::as_str),
                flags.get("pey").map(String::as_str),
            )? {
                Ok(format!("Tallerista {id} actualizado"))
            } else {
                Ok(format!("No hay tallerista con id {id}"))
            }
        }
        "cuota" => {
            let taller = numero(flags, "taller")?;
            let monto = numero(flags, "monto")?;
            match db::cambiar_cuota(conn, taller, monto)? {
                None => Ok(format!("No hay taller con id {taller}")),
                Some((anterior, nueva)) if anterior == nueva => {
                    Ok(format!("La cuota del taller {taller} ya era ${nueva} MXN"))
                }
                Some((anterior, nueva)) => Ok(format!(
                    "Cuota del taller {taller}: ${anterior} -> ${nueva} MXN"
                )),
            }
        }
        "inscripcion" => {
            let id = numero(flags, "id")?;
            let activa = numero(flags, "activa")?;
            if db::cambiar_inscripcion(conn, id, activa)? {
                Ok(format!("Inscripción {id} actualizada"))
            } else {
                Ok(format!("No hay inscripción con id {id}"))
            }
        }
        "pago" => {
            let id = numero(flags, "id")?;
            if !flags.contains_key("estado") && !flags.contains_key("monto") {
                return Err("indica --estado o --monto".to_string());
            }
            if db::cambiar_pago(
                conn,
                id,
                flags.get("estado").map(String::as_str),
                numero_opcional(flags, "monto")?,
            )? {
                Ok(format!("Pago {id} actualizado"))
            } else {
                Ok(format!("No hay pago con id {id}"))
            }
        }
        otro => Err(format!("no se puede cambiar '{otro}'")),
    }
}

fn borrar(
    conn: &Connection,
    que: Option<&str>,
    flags: &BTreeMap<String, String>,
) -> Result<String, String> {
    let nombre = que.ok_or("indica qué borrar, por ejemplo: borrar pagos")?;
    let tabla = match nombre {
        "alumno" | "alumnos" => "alumnos",
        "tallerista" | "talleristas" => "talleristas",
        "taller" | "talleres" => "talleres",
        "nfc" => "dispositivos_nfc",
        "anual" | "anuales" => "inscripciones_anuales",
        "inscripcion" | "inscripciones" => "inscripciones_taller",
        "pago" | "pagos" => "pagos",
        _ => return Err(format!("no se puede borrar '{nombre}'")),
    };
    let id = numero(flags, "id")?;
    if db::borrar(conn, tabla, id)? {
        Ok(format!("{nombre} {id} borrado"))
    } else {
        Ok(format!("No hay {nombre} con id {id}"))
    }
}

fn banderas(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut flags = BTreeMap::new();
    let mut indice = 0;
    while indice < args.len() {
        let arg = &args[indice];
        let Some(clave) = arg.strip_prefix("--") else {
            return Err(format!("opción no reconocida: {arg}"));
        };
        if clave.is_empty() {
            return Err("falta el nombre de la opción".to_string());
        }
        let valor = args
            .get(indice + 1)
            .filter(|valor| !valor.starts_with("--"))
            .ok_or_else(|| format!("falta el valor de --{clave}"))?;
        if flags.insert(clave.to_string(), valor.clone()).is_some() {
            return Err(format!("--{clave} está repetida"));
        }
        indice += 2;
    }
    Ok(flags)
}

fn obligatorio(flags: &BTreeMap<String, String>, clave: &str) -> Result<String, String> {
    flags
        .get(clave)
        .cloned()
        .ok_or_else(|| format!("falta --{clave}"))
}

fn numero(flags: &BTreeMap<String, String>, clave: &str) -> Result<i64, String> {
    let texto = obligatorio(flags, clave)?;
    texto
        .parse()
        .map_err(|_| format!("--{clave} debe ser un número entero"))
}

fn numero_opcional(flags: &BTreeMap<String, String>, clave: &str) -> Result<Option<i64>, String> {
    match flags.get(clave) {
        None => Ok(None),
        Some(texto) => texto
            .parse()
            .map(Some)
            .map_err(|_| format!("--{clave} debe ser un número entero")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::prepare(&mut conn).unwrap();
        conn
    }

    fn comando(linea: &[&str]) -> Vec<String> {
        linea.iter().map(|parte| (*parte).to_string()).collect()
    }

    #[test]
    fn agrega_cambia_y_borra_un_alumno() {
        let mut conn = base();
        let alta = ejecutar(
            &mut conn,
            &comando(&[
                "agregar",
                "alumno",
                "--nombre",
                "Lucía Méndez",
                "--cartera",
                "cartera_lucia",
            ]),
        )
        .unwrap();
        assert_eq!(alta, "Alumno 11 creado");

        let lista = ejecutar(&mut conn, &comando(&["listar", "alumnos"])).unwrap();
        assert!(lista.contains("Lucía Méndez"));

        let cambio = ejecutar(
            &mut conn,
            &comando(&["cambiar", "alumno", "--id", "11", "--nombre", "Lucía M."]),
        )
        .unwrap();
        assert_eq!(cambio, "Alumno 11 actualizado");

        let borrado = ejecutar(&mut conn, &comando(&["borrar", "alumnos", "--id", "11"])).unwrap();
        assert_eq!(borrado, "alumnos 11 borrado");
        let lista = ejecutar(&mut conn, &comando(&["listar", "alumnos"])).unwrap();
        assert!(!lista.contains("Lucía"));
    }

    #[test]
    fn no_borra_un_alumno_con_pagos() {
        let mut conn = base();
        let error = ejecutar(&mut conn, &comando(&["borrar", "alumnos", "--id", "1"])).unwrap_err();
        assert!(error.contains("ligado"));
    }

    #[test]
    fn cambia_la_cuota_y_guarda_el_anterior() {
        let mut conn = base();
        let aviso = ejecutar(
            &mut conn,
            &comando(&["cambiar", "cuota", "--taller", "1", "--monto", "30"]),
        )
        .unwrap();
        assert_eq!(aviso, "Cuota del taller 1: $25 -> $30 MXN");

        let talleres = ejecutar(&mut conn, &comando(&["listar", "talleres"])).unwrap();
        assert!(talleres.contains("1 | Marichuy | Tejido | 30 | 1"));
        let cuotas = ejecutar(&mut conn, &comando(&["listar", "cuotas"])).unwrap();
        assert!(cuotas.contains("| Tejido | 25 | 30 |"));
    }

    #[test]
    fn marca_un_pago_pendiente_como_pagado() {
        let mut conn = base();
        let pagos = ejecutar(&mut conn, &comando(&["listar", "pagos"])).unwrap();
        let id = pagos
            .lines()
            .find(|linea| linea.contains("Camila Flores") && linea.contains("pendiente"))
            .unwrap()
            .split(" | ")
            .next()
            .unwrap();
        let aviso = ejecutar(
            &mut conn,
            &comando(&["cambiar", "pago", "--id", id, "--estado", "pagado"]),
        )
        .unwrap();
        assert_eq!(aviso, format!("Pago {id} actualizado"));

        let pagos = ejecutar(&mut conn, &comando(&["listar", "pagos"])).unwrap();
        let linea = pagos
            .lines()
            .find(|linea| linea.starts_with(&format!("{id} |")))
            .unwrap();
        assert!(linea.contains("pagado"));
        let ingresos = ejecutar(&mut conn, &comando(&["listar", "ingresos"])).unwrap();
        assert!(ingresos.contains("Marichuy | tejido | 100"));
    }
}
