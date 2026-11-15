//! Páginas HTML del tallerista.

use crate::services::{AlumnoCuenta, Cuenta, TallerCuenta};

pub fn eleccion() -> String {
	documento(
		"Cuentas Conmigo",
		r#"<h1>Cuentas Conmigo</h1>
<p class="lead">Elige qué quieres hacer.</p>
<div class="opciones">
<a class="boton" href="/entrar">Iniciar sesión</a>
<a class="boton secundario" href="/crear">Crear una cuenta</a>
</div>"#,
	)
}

pub fn entrar(error: Option<&str>) -> String {
	documento(
		"Iniciar sesión",
		&format!(
			r#"<p class="paso">Ya tengo cuenta</p>
<h1>Iniciar sesión</h1>
{error}
<form method="post" action="/entrar">
<label for="usuario">Usuario</label>
<input id="usuario" name="usuario" autocomplete="username" required>
<label for="clave">Contraseña</label>
<input id="clave" name="clave" type="password" autocomplete="current-password" required>
<button type="submit">Entrar</button>
</form>
<p class="cambio"><a href="/crear">Crear una cuenta</a> · <a href="/">Volver</a></p>"#,
			error = aviso(error),
		),
	)
}

pub fn crear(error: Option<&str>) -> String {
	documento(
		"Crear una cuenta",
		&format!(
			r#"<p class="paso">Cuenta nueva</p>
<h1>Crear una cuenta</h1>
<p class="lead">Después te pedimos tu nombre, el oficio, el taller y el horario.</p>
{error}
<form method="post" action="/crear">
<label for="usuario">Usuario</label>
<input id="usuario" name="usuario" autocomplete="username" required>
<label for="clave">Contraseña</label>
<input id="clave" name="clave" type="password" autocomplete="new-password" minlength="6" required>
<button type="submit">Crear cuenta</button>
</form>
<p class="cambio"><a href="/entrar">Ya tengo cuenta</a> · <a href="/">Volver</a></p>"#,
			error = aviso(error),
		),
	)
}

pub fn cuenta(cuenta: &Cuenta, error: Option<&str>) -> String {
	let cuerpo = if cuenta.oficio.trim().is_empty() {
		paso_datos(cuenta, error)
	} else if cuenta.talleres.is_empty() {
		paso_taller(error)
	} else if cuenta.talleres.iter().any(|taller| taller.horarios.is_empty()) {
		paso_horario(cuenta, error)
	} else if cuenta.talleres.iter().all(|taller| taller.alumnos.is_empty()) {
		paso_alumno(cuenta, error)
	} else {
		panel(cuenta, error)
	};
	documento("Tu cuenta", &format!("{}{}", barra(cuenta), cuerpo))
}

fn paso_datos(cuenta: &Cuenta, error: Option<&str>) -> String {
	format!(
		r#"<p class="paso">Paso 1 de 4</p>
<h1>Tu nombre y tu oficio</h1>
{error}
<form method="post" action="/cuenta/datos">
<label for="nombre">Nombre</label>
<input id="nombre" name="nombre" value="{nombre}" autocomplete="name" required>
<label for="oficio">Oficio</label>
<input id="oficio" name="oficio" autocomplete="organization-title" required>
<button type="submit">Continuar</button>
</form>"#,
		error = aviso(error),
		nombre = escapar(&cuenta.tallerista),
	)
}

fn paso_taller(error: Option<&str>) -> String {
	format!(
		r#"<p class="paso">Paso 2 de 4</p>
<h1>Tu taller</h1>
{error}
<form method="post" action="/cuenta/taller">
<label for="nombre">Nombre del taller</label>
<input id="nombre" name="nombre" required>
<label for="cuota">Cuota al mes, en pesos</label>
<input id="cuota" name="cuota_mensual_mxn" type="number" min="1" value="25" required>
<button type="submit">Continuar</button>
</form>"#,
		error = aviso(error),
	)
}

fn paso_horario(cuenta: &Cuenta, error: Option<&str>) -> String {
	let pendientes: Vec<&TallerCuenta> = cuenta
		.talleres
		.iter()
		.filter(|taller| taller.horarios.is_empty())
		.collect();
	format!(
		r#"<p class="paso">Paso 3 de 4</p>
<h1>El horario</h1>
{error}
<form method="post" action="/cuenta/horario">
{taller}
<label for="dia">Día</label>
<select id="dia" name="dia" required>
<option value="lunes">lunes</option>
<option value="martes" selected>martes</option>
<option value="miercoles">miércoles</option>
<option value="jueves">jueves</option>
<option value="viernes">viernes</option>
<option value="sabado">sábado</option>
<option value="domingo">domingo</option>
</select>
<label for="de">Desde</label>
<input id="de" name="de" type="time" value="16:00" required>
<label for="a">Hasta</label>
<input id="a" name="a" type="time" value="18:00" required>
<button type="submit">Continuar</button>
</form>"#,
		error = aviso(error),
		taller = campo_taller(&pendientes, "taller"),
	)
}

fn paso_alumno(cuenta: &Cuenta, error: Option<&str>) -> String {
	format!(
		r#"<p class="paso">Paso 4 de 4</p>
<h1>Tu primer alumno</h1>
{error}
<form method="post" action="/cuenta/alumno">
{taller}
<label for="alumno">Nombre del alumno</label>
<input id="alumno" name="nombre" autocomplete="name" required>
<button type="submit">Guardar alumno</button>
</form>"#,
		error = aviso(error),
		taller = campo_taller(&cuenta.talleres.iter().collect::<Vec<_>>(), "taller"),
	)
}

fn panel(cuenta: &Cuenta, error: Option<&str>) -> String {
	let mut talleres = String::new();
	for taller in &cuenta.talleres {
		talleres.push_str(&taller_html(taller));
	}
	format!(
		r#"{error}
<h1>Tus talleres</h1>
{talleres}
<h2>Agregar un alumno</h2>
<form method="post" action="/cuenta/alumno">
{taller}
<label for="alumno">Nombre del alumno</label>
<input id="alumno" name="nombre" autocomplete="name" required>
<button type="submit">Guardar alumno</button>
</form>
<h2>Otro horario</h2>
<form method="post" action="/cuenta/horario">
{taller_horario}
<label for="dia">Día</label>
<select id="dia" name="dia" required>
<option value="lunes">lunes</option>
<option value="martes">martes</option>
<option value="miercoles">miércoles</option>
<option value="jueves">jueves</option>
<option value="viernes">viernes</option>
<option value="sabado">sábado</option>
<option value="domingo">domingo</option>
</select>
<label for="de">Desde</label>
<input id="de" name="de" type="time" value="16:00" required>
<label for="a">Hasta</label>
<input id="a" name="a" type="time" value="18:00" required>
<button type="submit">Guardar horario</button>
</form>"#,
		error = aviso(error),
		taller = campo_taller(&cuenta.talleres.iter().collect::<Vec<_>>(), "taller-alumno"),
		taller_horario = campo_taller(&cuenta.talleres.iter().collect::<Vec<_>>(), "taller-horario"),
	)
}

fn taller_html(taller: &TallerCuenta) -> String {
	let mut horarios = String::new();
	for horario in &taller.horarios {
		horarios.push_str(&format!(
			"<li>{} · {} a {}</li>",
			escapar(&horario.dia),
			escapar(&horario.de),
			escapar(&horario.a)
		));
	}
	let mut alumnos = String::new();
	for alumno in &taller.alumnos {
		alumnos.push_str(&alumno_html(&taller.nombre, alumno));
	}
	if alumnos.is_empty() {
		alumnos.push_str("<li>Todavía no hay alumnos.</li>");
	}
	format!(
		r#"<section class="taller">
<h2>{nombre}</h2>
<p>{cuota} pesos al mes</p>
<ul>{horarios}</ul>
<h3>Alumnos</h3>
<ul class="alumnos">{alumnos}</ul>
</section>"#,
		nombre = escapar(&taller.nombre),
		cuota = taller.cuota_mensual_mxn,
	)
}

fn alumno_html(taller: &str, alumno: &AlumnoCuenta) -> String {
	let estado = match alumno.mensualidad.as_str() {
		"pagado" => "Pagado".to_string(),
		"fallido" => "No se pudo cobrar".to_string(),
		_ => "Pendiente".to_string(),
	};
	let accion = if alumno.mensualidad == "pagado" {
		String::new()
	} else {
		format!(
			r#"<form method="post" action="/cuenta/pago">
<input type="hidden" name="alumno" value="{alumno}">
<input type="hidden" name="taller" value="{taller}">
<input type="hidden" name="estado" value="pagado">
<button type="submit">Ya pagó</button>
</form>"#,
			alumno = escapar(&alumno.nombre),
			taller = escapar(taller),
		)
	};
	format!("<li><span>{} · {}</span>{accion}</li>", escapar(&alumno.nombre), estado)
}

fn campo_taller(talleres: &[&TallerCuenta], id: &str) -> String {
	if talleres.len() <= 1 {
		let nombre = talleres.first().map(|taller| taller.nombre.as_str()).unwrap_or("");
		return format!(r#"<input type="hidden" name="taller" value="{}">"#, escapar(nombre));
	}
	let mut opciones = String::new();
	for taller in talleres {
		opciones.push_str(&format!(
			r#"<option value="{}">{}</option>"#,
			escapar(&taller.nombre),
			escapar(&taller.nombre)
		));
	}
	format!(r#"<label for="{id}">Taller</label><select id="{id}" name="taller" required>{opciones}</select>"#)
}

fn barra(cuenta: &Cuenta) -> String {
	format!(
		r#"<header class="barra">
<p>Hola, {nombre}</p>
<form method="post" action="/salir"><button class="salir" type="submit">Salir</button></form>
</header>"#,
		nombre = escapar(&cuenta.tallerista),
	)
}

fn aviso(error: Option<&str>) -> String {
	match error {
		Some(error) if !error.is_empty() => {
			format!(r#"<p class="error">{}</p>"#, escapar(error))
		}
		_ => String::new(),
	}
}

fn escapar(valor: &str) -> String {
	let mut texto = String::new();
	for caracter in valor.chars() {
		match caracter {
			'&' => texto.push_str("&amp;"),
			'<' => texto.push_str("&lt;"),
			'>' => texto.push_str("&gt;"),
			'"' => texto.push_str("&quot;"),
			'\'' => texto.push_str("&#39;"),
			otro => texto.push(otro),
		}
	}
	texto
}

fn documento(titulo: &str, cuerpo: &str) -> String {
	format!(
		r#"<!DOCTYPE html>
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{titulo}</title>
<style>
body {{ margin: 0; background: #f6f3ee; color: #1c1917; font: 1.2rem/1.45 system-ui, sans-serif; }}
main {{ max-width: 32rem; margin: 0 auto; padding: 2rem 1.25rem 3rem; }}
h1 {{ font-size: 2rem; line-height: 1.2; margin: 0 0 1rem; }}
h2 {{ font-size: 1.4rem; margin: 2rem 0 0.5rem; }}
h3 {{ font-size: 1.1rem; margin: 1rem 0 0.4rem; }}
p {{ margin: 0 0 1rem; }}
.lead {{ color: #44403c; }}
.paso {{ margin: 0 0 0.4rem; color: #57534e; font-size: 1rem; }}
.opciones {{ display: grid; gap: 0.75rem; }}
a.boton, button {{ display: block; width: 100%; box-sizing: border-box; border: 0; border-radius: 0.8rem; background: #9a3412; color: white; font: inherit; font-weight: 650; text-align: center; text-decoration: none; padding: 0.9rem 1rem; cursor: pointer; }}
a.secundario, button.salir {{ background: #44403c; }}
label {{ display: block; margin: 1rem 0 0.35rem; font-weight: 650; }}
input, select {{ width: 100%; box-sizing: border-box; border: 2px solid #d6d3d1; border-radius: 0.7rem; background: white; color: inherit; font: inherit; padding: 0.75rem 0.8rem; }}
form button {{ margin-top: 1.25rem; }}
.error {{ background: #fee2e2; border-radius: 0.7rem; padding: 0.8rem 1rem; }}
.cambio {{ margin-top: 1.5rem; }}
.cambio a, header a {{ color: #9a3412; }}
.barra {{ display: flex; justify-content: space-between; align-items: center; gap: 1rem; margin-bottom: 1.5rem; }}
.barra p {{ margin: 0; font-weight: 650; }}
.barra form {{ margin: 0; }}
button.salir {{ width: auto; padding: 0.55rem 0.9rem; }}
.taller {{ background: white; border-radius: 0.9rem; padding: 1rem 1rem 0.4rem; margin: 0 0 1rem; }}
ul {{ padding-left: 1.2rem; }}
.alumnos li {{ margin: 0 0 0.8rem; }}
.alumnos form button {{ margin-top: 0.4rem; }}
</style>
</head>
<body>
<main>
{cuerpo}
</main>
</body>
</html>"#,
		titulo = escapar(titulo),
	)
}
