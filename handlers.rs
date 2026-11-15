use actix_web::{
    HttpRequest, HttpResponse, Responder, ResponseError, delete, get, http::StatusCode, post, put,
    web,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::Deserialize;

use crate::db;
use crate::entity::{alumno, anual, cambio_cuota, inscripcion, nfc, pago, taller, tallerista};
use crate::pagina;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(inicio)
        .service(resumen)
        .service(listar_alumnos)
        .service(crear_alumno)
        .service(cambiar_alumno)
        .service(borrar_alumno)
        .service(listar_talleristas)
        .service(crear_tallerista)
        .service(cambiar_tallerista)
        .service(borrar_tallerista)
        .service(listar_talleres)
        .service(crear_taller)
        .service(cambiar_cuota)
        .service(borrar_taller)
        .service(listar_nfc)
        .service(crear_nfc)
        .service(borrar_nfc)
        .service(listar_anuales)
        .service(crear_anual)
        .service(borrar_anual)
        .service(listar_inscripciones)
        .service(crear_inscripcion)
        .service(cambiar_inscripcion)
        .service(borrar_inscripcion)
        .service(listar_pagos)
        .service(crear_pago)
        .service(cambiar_pago)
        .service(borrar_pago)
        .service(listar_cuotas)
        .service(listar_ingresos)
        .service(alumno_agregar_navegador)
        .service(alumno_cambiar_navegador)
        .service(alumno_borrar_navegador)
        .service(tallerista_agregar_navegador)
        .service(tallerista_cambiar_navegador)
        .service(tallerista_borrar_navegador)
        .service(taller_agregar_navegador)
        .service(taller_cuota_navegador)
        .service(taller_borrar_navegador)
        .service(nfc_agregar_navegador)
        .service(nfc_borrar_navegador)
        .service(anual_agregar_navegador)
        .service(anual_borrar_navegador)
        .service(inscripcion_agregar_navegador)
        .service(inscripcion_cambiar_navegador)
        .service(inscripcion_borrar_navegador)
        .service(pago_agregar_navegador)
        .service(pago_cambiar_navegador)
        .service(pago_borrar_navegador)
        .service(cuenta)
        .service(pagina_entrar)
        .service(entrar_formulario)
        .service(pagina_crear)
        .service(crear_formulario)
        .service(salir)
        .service(crear_cuenta_navegador)
        .service(entrar_cuenta_navegador)
        .service(datos_cuenta_navegador)
        .service(datos_cuenta_formulario)
        .service(taller_cuenta_navegador)
        .service(taller_cuenta_formulario)
        .service(horario_cuenta_navegador)
        .service(horario_cuenta_formulario)
        .service(alumno_cuenta_navegador)
        .service(alumno_cuenta_formulario)
        .service(pago_cuenta_navegador)
        .service(pago_cuenta_formulario);
}

#[get("/")]
async fn inicio() -> impl Responder {
    html(pagina::eleccion())
}

#[get("/entrar")]
async fn pagina_entrar() -> impl Responder {
    html(pagina::entrar(None))
}

#[post("/entrar")]
async fn entrar_formulario(
    db: web::Data<DatabaseConnection>,
    form: web::Form<Credenciales>,
) -> Result<HttpResponse, ApiError> {
    match db::entrar_con_clave(db.get_ref(), &form.usuario, &form.clave).await {
        Ok(entrada) => Ok(ir_a_cuenta(Some(&entrada.token))),
        Err(error) => Ok(html_estado(
            StatusCode::BAD_REQUEST,
            pagina::entrar(Some(&texto_aviso(error))),
        )),
    }
}

#[get("/crear")]
async fn pagina_crear() -> impl Responder {
    html(pagina::crear(None))
}

#[post("/crear")]
async fn crear_formulario(
    db: web::Data<DatabaseConnection>,
    form: web::Form<Credenciales>,
) -> Result<HttpResponse, ApiError> {
    match db::crear_cuenta(db.get_ref(), &form.usuario, &form.clave).await {
        Ok(entrada) => Ok(ir_a_cuenta(Some(&entrada.token))),
        Err(error) => Ok(html_estado(
            StatusCode::BAD_REQUEST,
            pagina::crear(Some(&texto_aviso(error))),
        )),
    }
}

#[post("/salir")]
async fn salir() -> impl Responder {
    let mut galleta = cookie_sesion("");
    galleta.make_removal();
    HttpResponse::SeeOther()
        .insert_header((actix_web::http::header::LOCATION, "/"))
        .cookie(galleta)
        .finish()
}

#[get("/cuenta")]
async fn cuenta(
    req: HttpRequest,
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
) -> Result<HttpResponse, ApiError> {
    let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
    if pide_html(&req) {
        Ok(html(pagina::cuenta(&cuerpo, None)))
    } else {
        Ok(HttpResponse::Ok().json(cuerpo))
    }
}

#[derive(Deserialize)]
struct Credenciales {
    usuario: String,
    clave: String,
}

#[get("/cuenta/crear")]
async fn crear_cuenta_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<Credenciales>,
) -> Result<HttpResponse, ApiError> {
    let entrada = db::crear_cuenta(db.get_ref(), &query.usuario, &query.clave)
        .await
        .map_err(ApiError::db)?;
    responder_cuenta(db.get_ref(), &entrada, StatusCode::CREATED).await
}

#[get("/cuenta/entrar")]
async fn entrar_cuenta_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<Credenciales>,
) -> Result<HttpResponse, ApiError> {
    let entrada = db::entrar_con_clave(db.get_ref(), &query.usuario, &query.clave)
        .await
        .map_err(ApiError::db)?;
    responder_cuenta(db.get_ref(), &entrada, StatusCode::OK).await
}

async fn responder_cuenta(
    db: &DatabaseConnection,
    entrada: &db::Entrada,
    status: StatusCode,
) -> Result<HttpResponse, ApiError> {
    let cuerpo = db::cuenta(db, &entrada.sesion).await.map_err(ApiError::db)?;
    Ok(HttpResponse::build(status)
        .cookie(cookie_sesion(&entrada.token))
        .json(cuerpo))
}

fn cookie_sesion(token: &str) -> actix_web::cookie::Cookie<'static> {
    actix_web::cookie::Cookie::build("sesion", token.to_owned())
        .path("/")
        .http_only(true)
        .same_site(actix_web::cookie::SameSite::Lax)
        .finish()
}

fn html(cuerpo: String) -> HttpResponse {
    html_estado(StatusCode::OK, cuerpo)
}

fn html_estado(status: StatusCode, cuerpo: String) -> HttpResponse {
    HttpResponse::build(status)
        .content_type("text/html; charset=utf-8")
        .body(cuerpo)
}

fn ir_a_cuenta(token: Option<&str>) -> HttpResponse {
    let mut respuesta = HttpResponse::SeeOther();
    respuesta.insert_header((actix_web::http::header::LOCATION, "/cuenta"));
    if let Some(token) = token {
        respuesta.cookie(cookie_sesion(token));
    }
    respuesta.finish()
}

fn pide_html(req: &HttpRequest) -> bool {
    req.headers()
        .get(actix_web::http::header::ACCEPT)
        .and_then(|valor| valor.to_str().ok())
        .is_some_and(|valor| valor.contains("text/html"))
}

fn texto_aviso(error: DbErr) -> String {
    let texto = error.to_string();
    texto
        .split_once("aviso: ")
        .map(|(_, aviso)| aviso.to_string())
        .unwrap_or_else(|| "No se pudo guardar. Inténtalo de nuevo.".to_string())
}

async fn cuenta_con_error(
    db: &DatabaseConnection,
    sesion: &db::Sesion,
    error: DbErr,
) -> Result<HttpResponse, ApiError> {
    let cuerpo = db::cuenta(db, sesion).await.map_err(ApiError::db)?;
    Ok(html_estado(
        StatusCode::BAD_REQUEST,
        pagina::cuenta(&cuerpo, Some(&texto_aviso(error))),
    ))
}

#[derive(Deserialize)]
struct DatosCuenta {
    nombre: String,
    oficio: String,
}

#[get("/cuenta/datos")]
async fn datos_cuenta_navegador(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    query: web::Query<DatosCuenta>,
) -> Result<HttpResponse, ApiError> {
    db::guardar_datos_cuenta(db.get_ref(), sesion.id, &query.nombre, &query.oficio)
        .await
        .map_err(ApiError::db)?;
    let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(cuerpo))
}

#[derive(Deserialize)]
struct TallerDeCuenta {
    nombre: String,
    cuota_mensual_mxn: Option<i32>,
}

#[get("/cuenta/taller")]
async fn taller_cuenta_navegador(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    query: web::Query<TallerDeCuenta>,
) -> Result<HttpResponse, ApiError> {
    let cuota = query.cuota_mensual_mxn.unwrap_or(db::MENSUALIDAD_MXN);
    db::guardar_taller_cuenta(db.get_ref(), sesion.id, &query.nombre, cuota)
        .await
        .map_err(ApiError::db)?;
    let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(cuerpo))
}

#[derive(Deserialize)]
struct HorarioDeCuenta {
    taller: Option<String>,
    dia: String,
    de: String,
    a: String,
}

#[get("/cuenta/horario")]
async fn horario_cuenta_navegador(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    query: web::Query<HorarioDeCuenta>,
) -> Result<HttpResponse, ApiError> {
    db::guardar_horario_cuenta(
        db.get_ref(),
        sesion.id,
        query.taller.as_deref(),
        &query.dia,
        &query.de,
        &query.a,
    )
    .await
    .map_err(ApiError::db)?;
    let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(cuerpo))
}

#[derive(Deserialize)]
struct AlumnoDeCuenta {
    nombre: String,
    taller: Option<String>,
}

#[get("/cuenta/alumno")]
async fn alumno_cuenta_navegador(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    query: web::Query<AlumnoDeCuenta>,
) -> Result<HttpResponse, ApiError> {
    db::guardar_alumno_cuenta(
        db.get_ref(),
        sesion.id,
        &query.nombre,
        query.taller.as_deref(),
    )
    .await
    .map_err(ApiError::db)?;
    let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(cuerpo))
}

#[derive(Deserialize)]
struct PagoDeCuenta {
    alumno: String,
    taller: Option<String>,
    estado: Option<String>,
}

#[get("/cuenta/pago")]
async fn pago_cuenta_navegador(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    query: web::Query<PagoDeCuenta>,
) -> Result<HttpResponse, ApiError> {
    let estado = query.estado.as_deref().unwrap_or("pagado");
    db::marcar_pago_cuenta(
        db.get_ref(),
        sesion.id,
        &query.alumno,
        query.taller.as_deref(),
        estado,
    )
    .await
    .map_err(ApiError::db)?;
    let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(cuerpo))
}

#[post("/cuenta/datos")]
async fn datos_cuenta_formulario(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    form: web::Form<DatosCuenta>,
) -> Result<HttpResponse, ApiError> {
    match db::guardar_datos_cuenta(db.get_ref(), sesion.id, &form.nombre, &form.oficio).await {
        Ok(()) => Ok(ir_a_cuenta(None)),
        Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
    }
}

#[post("/cuenta/taller")]
async fn taller_cuenta_formulario(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    form: web::Form<TallerDeCuenta>,
) -> Result<HttpResponse, ApiError> {
    let cuota = form.cuota_mensual_mxn.unwrap_or(db::MENSUALIDAD_MXN);
    match db::guardar_taller_cuenta(db.get_ref(), sesion.id, &form.nombre, cuota).await {
        Ok(()) => Ok(ir_a_cuenta(None)),
        Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
    }
}

#[post("/cuenta/horario")]
async fn horario_cuenta_formulario(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    form: web::Form<HorarioDeCuenta>,
) -> Result<HttpResponse, ApiError> {
    match db::guardar_horario_cuenta(
        db.get_ref(),
        sesion.id,
        form.taller.as_deref(),
        &form.dia,
        &form.de,
        &form.a,
    )
    .await
    {
        Ok(()) => Ok(ir_a_cuenta(None)),
        Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
    }
}

#[post("/cuenta/alumno")]
async fn alumno_cuenta_formulario(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    form: web::Form<AlumnoDeCuenta>,
) -> Result<HttpResponse, ApiError> {
    match db::guardar_alumno_cuenta(
        db.get_ref(),
        sesion.id,
        &form.nombre,
        form.taller.as_deref(),
    )
    .await
    {
        Ok(()) => Ok(ir_a_cuenta(None)),
        Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
    }
}

#[post("/cuenta/pago")]
async fn pago_cuenta_formulario(
    db: web::Data<DatabaseConnection>,
    sesion: web::ReqData<db::Sesion>,
    form: web::Form<PagoDeCuenta>,
) -> Result<HttpResponse, ApiError> {
    let estado = form.estado.clone().unwrap_or_else(|| "pagado".to_string());
    match db::marcar_pago_cuenta(
        db.get_ref(),
        sesion.id,
        &form.alumno,
        form.taller.as_deref(),
        &estado,
    )
    .await
    {
        Ok(()) => Ok(ir_a_cuenta(None)),
        Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
    }
}

#[get("/resumen")]
async fn resumen(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
    let ingresos = db::ingresos(db.get_ref()).await.map_err(ApiError::db)?;
    let pagos_pagados = pago::Entity::find()
        .filter(pago::Column::Estado.eq("pagado"))
        .count(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    let pagos_pendientes = pago::Entity::find()
        .filter(pago::Column::Estado.eq("pendiente"))
        .count(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "inscripcion_anual_mxn": db::INSCRIPCION_ANUAL_MXN,
        "mensualidad_mxn": db::MENSUALIDAD_MXN,
        "talleristas": tallerista::Entity::find().count(db.get_ref()).await.map_err(ApiError::db)?,
        "alumnos": alumno::Entity::find().count(db.get_ref()).await.map_err(ApiError::db)?,
        "inscripciones": inscripcion::Entity::find().count(db.get_ref()).await.map_err(ApiError::db)?,
        "pagos_pagados": pagos_pagados,
        "pagos_pendientes": pagos_pendientes,
        "ingresos": ingresos,
    })))
}

#[get("/alumnos")]
async fn listar_alumnos(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
    let filas = alumno::Entity::find()
        .order_by_asc(alumno::Column::Id)
        .all(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(filas))
}

#[derive(Deserialize)]
struct NuevoAlumno {
    nombre: String,
    cartera_ref: String,
}

#[post("/alumnos")]
async fn crear_alumno(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoAlumno>,
) -> Result<HttpResponse, ApiError> {
    guardar_alumno(db, body).await
}

async fn guardar_alumno(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoAlumno>,
) -> Result<HttpResponse, ApiError> {
    exigir_texto(&body.nombre, "nombre")?;
    exigir_texto(&body.cartera_ref, "cartera_ref")?;
    let modelo = alumno::ActiveModel {
        nombre: Set(body.nombre.trim().to_string()),
        cartera_ref: Set(body.cartera_ref.trim().to_string()),
        ..Default::default()
    };
    let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
struct CambioAlumno {
    nombre: Option<String>,
    cartera_ref: Option<String>,
}

#[put("/alumnos/{id}")]
async fn cambiar_alumno(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<CambioAlumno>,
) -> Result<HttpResponse, ApiError> {
    guardar_cambio_alumno(db, id, body).await
}

async fn guardar_cambio_alumno(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<CambioAlumno>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let Some(actual) = alumno::Entity::find_by_id(id)
        .one(db.get_ref())
        .await
        .map_err(ApiError::db)?
    else {
        return Err(ApiError::no("alumno", id));
    };
    let mut activo: alumno::ActiveModel = actual.into();
    if let Some(nombre) = &body.nombre {
        exigir_texto(nombre, "nombre")?;
        activo.nombre = Set(nombre.trim().to_string());
    }
    if let Some(cartera) = &body.cartera_ref {
        exigir_texto(cartera, "cartera_ref")?;
        activo.cartera_ref = Set(cartera.trim().to_string());
    }
    if body.nombre.is_none() && body.cartera_ref.is_none() {
        return Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "indica nombre o cartera_ref",
        ));
    }
    let actualizado = activo.update(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(actualizado))
}

#[delete("/alumnos/{id}")]
async fn borrar_alumno(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    eliminar::<alumno::Entity>(db.get_ref(), id.into_inner(), "alumno").await
}

#[get("/talleristas")]
async fn listar_talleristas(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
    let filas = tallerista::Entity::find()
        .order_by_asc(tallerista::Column::Id)
        .all(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(filas))
}

#[derive(Deserialize)]
struct NuevoTallerista {
    nombre: String,
    oficio: String,
    pey_api_key_id: String,
}

#[post("/talleristas")]
async fn crear_tallerista(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoTallerista>,
) -> Result<HttpResponse, ApiError> {
    guardar_tallerista(db, body).await
}

async fn guardar_tallerista(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoTallerista>,
) -> Result<HttpResponse, ApiError> {
    exigir_texto(&body.nombre, "nombre")?;
    exigir_texto(&body.oficio, "oficio")?;
    exigir_texto(&body.pey_api_key_id, "pey_api_key_id")?;
    let modelo = tallerista::ActiveModel {
        nombre: Set(body.nombre.trim().to_string()),
        oficio: Set(body.oficio.trim().to_string()),
        pey_api_key_id: Set(body.pey_api_key_id.trim().to_string()),
        ..Default::default()
    };
    let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
struct CambioTallerista {
    nombre: Option<String>,
    oficio: Option<String>,
    pey_api_key_id: Option<String>,
}

#[put("/talleristas/{id}")]
async fn cambiar_tallerista(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<CambioTallerista>,
) -> Result<HttpResponse, ApiError> {
    guardar_cambio_tallerista(db, id, body).await
}

async fn guardar_cambio_tallerista(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<CambioTallerista>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let Some(actual) = tallerista::Entity::find_by_id(id)
        .one(db.get_ref())
        .await
        .map_err(ApiError::db)?
    else {
        return Err(ApiError::no("tallerista", id));
    };
    if body.nombre.is_none() && body.oficio.is_none() && body.pey_api_key_id.is_none() {
        return Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "indica nombre, oficio o pey_api_key_id",
        ));
    }
    let mut activo: tallerista::ActiveModel = actual.into();
    if let Some(nombre) = &body.nombre {
        exigir_texto(nombre, "nombre")?;
        activo.nombre = Set(nombre.trim().to_string());
    }
    if let Some(oficio) = &body.oficio {
        exigir_texto(oficio, "oficio")?;
        activo.oficio = Set(oficio.trim().to_string());
    }
    if let Some(pey) = &body.pey_api_key_id {
        exigir_texto(pey, "pey_api_key_id")?;
        activo.pey_api_key_id = Set(pey.trim().to_string());
    }
    let actualizado = activo.update(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(actualizado))
}

#[delete("/talleristas/{id}")]
async fn borrar_tallerista(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    eliminar::<tallerista::Entity>(db.get_ref(), id.into_inner(), "tallerista").await
}

#[get("/talleres")]
async fn listar_talleres(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
    let filas = taller::Entity::find()
        .order_by_asc(taller::Column::Id)
        .all(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(filas))
}

#[derive(Deserialize)]
struct NuevoTaller {
    tallerista_id: i32,
    nombre: String,
    cuota_mensual_mxn: Option<i32>,
}

#[post("/talleres")]
async fn crear_taller(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoTaller>,
) -> Result<HttpResponse, ApiError> {
    guardar_taller(db, body).await
}

async fn guardar_taller(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoTaller>,
) -> Result<HttpResponse, ApiError> {
    exigir_texto(&body.nombre, "nombre")?;
    let cuota = body.cuota_mensual_mxn.unwrap_or(db::MENSUALIDAD_MXN);
    exigir_monto(cuota)?;
    let txn = db.get_ref().begin().await.map_err(ApiError::db)?;
    let modelo = taller::ActiveModel {
        tallerista_id: Set(body.tallerista_id),
        nombre: Set(body.nombre.trim().to_string()),
        cuota_mensual_mxn: Set(cuota),
        activo: Set(1),
        ..Default::default()
    };
    let creado = modelo.insert(&txn).await.map_err(ApiError::db)?;
    cambio_cuota::ActiveModel {
        taller_id: Set(creado.id),
        cuota_anterior_mxn: Set(None),
        cuota_nueva_mxn: Set(cuota),
        vigente_desde: Set(db::ahora(db.get_ref()).await.map_err(ApiError::db)?),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(ApiError::db)?;
    txn.commit().await.map_err(ApiError::db)?;
    Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
struct NuevaCuota {
    cuota_mensual_mxn: i32,
}

#[put("/talleres/{id}/cuota")]
async fn cambiar_cuota(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<NuevaCuota>,
) -> Result<HttpResponse, ApiError> {
    guardar_cuota(db, id, body).await
}

async fn guardar_cuota(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<NuevaCuota>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    exigir_monto(body.cuota_mensual_mxn)?;
    let txn = db.get_ref().begin().await.map_err(ApiError::db)?;
    let Some(actual) = taller::Entity::find_by_id(id)
        .one(&txn)
        .await
        .map_err(ApiError::db)?
    else {
        return Err(ApiError::no("taller", id));
    };
    if actual.cuota_mensual_mxn == body.cuota_mensual_mxn {
        return Ok(HttpResponse::Ok().json(actual));
    }
    let anterior = actual.cuota_mensual_mxn;
    let mut activo: taller::ActiveModel = actual.into();
    activo.cuota_mensual_mxn = Set(body.cuota_mensual_mxn);
    let actualizado = activo.update(&txn).await.map_err(ApiError::db)?;
    cambio_cuota::ActiveModel {
        taller_id: Set(id),
        cuota_anterior_mxn: Set(Some(anterior)),
        cuota_nueva_mxn: Set(body.cuota_mensual_mxn),
        vigente_desde: Set(db::ahora(db.get_ref()).await.map_err(ApiError::db)?),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(ApiError::db)?;
    txn.commit().await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(actualizado))
}

#[delete("/talleres/{id}")]
async fn borrar_taller(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    eliminar::<taller::Entity>(db.get_ref(), id.into_inner(), "taller").await
}

#[get("/nfc")]
async fn listar_nfc(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
    let filas = nfc::Entity::find()
        .order_by_asc(nfc::Column::Id)
        .all(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(filas))
}

#[derive(Deserialize)]
struct NuevoNfc {
    tallerista_id: i32,
    cara: String,
    chip_uid: String,
}

#[post("/nfc")]
async fn crear_nfc(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoNfc>,
) -> Result<HttpResponse, ApiError> {
    guardar_nfc(db, body).await
}

async fn guardar_nfc(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoNfc>,
) -> Result<HttpResponse, ApiError> {
    exigir_cara(&body.cara)?;
    exigir_texto(&body.chip_uid, "chip_uid")?;
    let modelo = nfc::ActiveModel {
        tallerista_id: Set(body.tallerista_id),
        cara: Set(body.cara.clone()),
        chip_uid: Set(body.chip_uid.trim().to_string()),
        ..Default::default()
    };
    let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Created().json(creado))
}

#[delete("/nfc/{id}")]
async fn borrar_nfc(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    eliminar::<nfc::Entity>(db.get_ref(), id.into_inner(), "nfc").await
}

#[get("/anuales")]
async fn listar_anuales(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
    let filas = anual::Entity::find()
        .order_by_asc(anual::Column::Id)
        .all(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(filas))
}

#[derive(Deserialize)]
struct NuevaAnual {
    alumno_id: i32,
    anio: Option<i32>,
    monto_mxn: Option<i32>,
}

#[post("/anuales")]
async fn crear_anual(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevaAnual>,
) -> Result<HttpResponse, ApiError> {
    guardar_anual(db, body).await
}

async fn guardar_anual(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevaAnual>,
) -> Result<HttpResponse, ApiError> {
    let monto = body.monto_mxn.unwrap_or(db::INSCRIPCION_ANUAL_MXN);
    exigir_monto(monto)?;
    let anio = match body.anio {
        Some(anio) => anio,
        None => db::anio_actual(db.get_ref()).await.map_err(ApiError::db)?,
    };
    let modelo = anual::ActiveModel {
        alumno_id: Set(body.alumno_id),
        anio: Set(anio),
        monto_mxn: Set(monto),
        ..Default::default()
    };
    let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Created().json(creado))
}

#[delete("/anuales/{id}")]
async fn borrar_anual(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    eliminar::<anual::Entity>(db.get_ref(), id.into_inner(), "inscripcion anual").await
}

#[get("/inscripciones")]
async fn listar_inscripciones(
    db: web::Data<DatabaseConnection>,
) -> Result<HttpResponse, ApiError> {
    let filas = inscripcion::Entity::find()
        .order_by_asc(inscripcion::Column::Id)
        .all(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(filas))
}

#[derive(Deserialize)]
struct NuevaInscripcion {
    alumno_id: i32,
    taller_id: i32,
    anio: Option<i32>,
}

#[post("/inscripciones")]
async fn crear_inscripcion(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevaInscripcion>,
) -> Result<HttpResponse, ApiError> {
    guardar_inscripcion(db, body).await
}

async fn guardar_inscripcion(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevaInscripcion>,
) -> Result<HttpResponse, ApiError> {
    let anio = match body.anio {
        Some(anio) => anio,
        None => db::anio_actual(db.get_ref()).await.map_err(ApiError::db)?,
    };
    let modelo = inscripcion::ActiveModel {
        alumno_id: Set(body.alumno_id),
        taller_id: Set(body.taller_id),
        anio: Set(anio),
        activa: Set(1),
        ..Default::default()
    };
    let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
struct CambioInscripcion {
    activa: i32,
}

#[put("/inscripciones/{id}")]
async fn cambiar_inscripcion(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<CambioInscripcion>,
) -> Result<HttpResponse, ApiError> {
    guardar_cambio_inscripcion(db, id, body).await
}

async fn guardar_cambio_inscripcion(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<CambioInscripcion>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    if body.activa != 0 && body.activa != 1 {
        return Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "activa debe ser 0 o 1",
        ));
    }
    let Some(actual) = inscripcion::Entity::find_by_id(id)
        .one(db.get_ref())
        .await
        .map_err(ApiError::db)?
    else {
        return Err(ApiError::no("inscripcion", id));
    };
    let mut activo: inscripcion::ActiveModel = actual.into();
    activo.activa = Set(body.activa);
    let actualizado = activo.update(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(actualizado))
}

#[delete("/inscripciones/{id}")]
async fn borrar_inscripcion(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    eliminar::<inscripcion::Entity>(db.get_ref(), id.into_inner(), "inscripcion").await
}

#[get("/pagos")]
async fn listar_pagos(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
    let filas = pago::Entity::find()
        .order_by_asc(pago::Column::Id)
        .all(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(filas))
}

#[derive(Deserialize)]
struct NuevoPago {
    alumno_id: i32,
    tipo: String,
    canal: String,
    periodo: String,
    taller_id: Option<i32>,
    monto_mxn: Option<i32>,
    num_ref: Option<String>,
    estado: Option<String>,
}

#[post("/pagos")]
async fn crear_pago(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoPago>,
) -> Result<HttpResponse, ApiError> {
    guardar_pago(db, body).await
}

async fn guardar_pago(
    db: web::Data<DatabaseConnection>,
    body: web::Json<NuevoPago>,
) -> Result<HttpResponse, ApiError> {
    exigir_tipo(&body.tipo)?;
    exigir_canal(&body.canal)?;
    exigir_texto(&body.periodo, "periodo")?;
    let estado = body
        .estado
        .clone()
        .unwrap_or_else(|| "pendiente".to_string());
    exigir_estado(&estado)?;

    let (taller_id, tallerista_id, concepto, cuota) = if body.tipo == "mensualidad" {
        let taller_id = body.taller_id.ok_or_else(|| {
            ApiError::mensaje(StatusCode::BAD_REQUEST, "la mensualidad necesita taller_id")
        })?;
        let Some(taller) = taller::Entity::find_by_id(taller_id)
            .one(db.get_ref())
            .await
            .map_err(ApiError::db)?
        else {
            return Err(ApiError::no("taller", taller_id));
        };
        (
            Some(taller_id),
            Some(taller.tallerista_id),
            format!("Mensualidad {} {}", taller.nombre, body.periodo.trim()),
            taller.cuota_mensual_mxn,
        )
    } else if body.taller_id.is_some() {
        return Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "la inscripcion anual no lleva taller_id",
        ));
    } else {
        (
            None,
            None,
            format!("Inscripción anual {}", body.periodo.trim()),
            db::INSCRIPCION_ANUAL_MXN,
        )
    };

    let monto = body.monto_mxn.unwrap_or(cuota);
    exigir_monto(monto)?;
    let num_ref = body
        .num_ref
        .as_deref()
        .map(str::trim)
        .filter(|valor| !valor.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "manual-{}-{}-{}-{}",
                body.tipo,
                body.alumno_id,
                body.periodo.trim(),
                marca_ms()
            )
        });
    let (transaction_id, tipo_evento, pagado_en) = if estado == "pagado" {
        (
            Some(format!("txn-{num_ref}")),
            Some("codi.qr.payment_received".to_string()),
            Some(db::ahora(db.get_ref()).await.map_err(ApiError::db)?),
        )
    } else {
        (None, None, None)
    };
    let modelo = pago::ActiveModel {
        alumno_id: Set(body.alumno_id),
        taller_id: Set(taller_id),
        tallerista_id: Set(tallerista_id),
        tipo: Set(body.tipo.clone()),
        canal: Set(body.canal.clone()),
        concepto: Set(concepto),
        monto_mxn: Set(monto),
        num_ref: Set(num_ref.clone()),
        codi_qr_id: Set(Some(format!("qr-{num_ref}"))),
        transaction_id: Set(transaction_id),
        tipo_evento: Set(tipo_evento),
        estado: Set(estado),
        periodo: Set(body.periodo.trim().to_string()),
        pagado_en: Set(pagado_en),
        ..Default::default()
    };
    let creado = modelo.insert(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Created().json(creado))
}

#[derive(Deserialize)]
struct CambioPago {
    estado: Option<String>,
    monto_mxn: Option<i32>,
}

#[put("/pagos/{id}")]
async fn cambiar_pago(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<CambioPago>,
) -> Result<HttpResponse, ApiError> {
    guardar_cambio_pago(db, id, body).await
}

async fn guardar_cambio_pago(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
    body: web::Json<CambioPago>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    if body.estado.is_none() && body.monto_mxn.is_none() {
        return Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "indica estado o monto_mxn",
        ));
    }
    if let Some(estado) = &body.estado {
        exigir_estado(estado)?;
    }
    if let Some(monto) = body.monto_mxn {
        exigir_monto(monto)?;
    }
    let Some(actual) = pago::Entity::find_by_id(id)
        .one(db.get_ref())
        .await
        .map_err(ApiError::db)?
    else {
        return Err(ApiError::no("pago", id));
    };
    let estado = body.estado.clone().unwrap_or(actual.estado.clone());
    let monto = body.monto_mxn.unwrap_or(actual.monto_mxn);
    let mut transaction_id = actual.transaction_id.clone();
    let mut tipo_evento = actual.tipo_evento.clone();
    let mut pagado_en = actual.pagado_en.clone();
    if estado == "pagado" {
        if transaction_id.is_none() {
            transaction_id = Some(format!("txn-manual-{id}"));
        }
        if tipo_evento.is_none() {
            tipo_evento = Some("codi.qr.payment_received".to_string());
        }
        if pagado_en.is_none() {
            pagado_en = Some(db::ahora(db.get_ref()).await.map_err(ApiError::db)?);
        }
    } else {
        pagado_en = None;
        tipo_evento = None;
    }
    let mut activo: pago::ActiveModel = actual.into();
    activo.estado = Set(estado);
    activo.monto_mxn = Set(monto);
    activo.transaction_id = Set(transaction_id);
    activo.tipo_evento = Set(tipo_evento);
    activo.pagado_en = Set(pagado_en);
    let actualizado = activo.update(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(actualizado))
}

#[delete("/pagos/{id}")]
async fn borrar_pago(
    db: web::Data<DatabaseConnection>,
    id: web::Path<i32>,
) -> Result<HttpResponse, ApiError> {
    eliminar::<pago::Entity>(db.get_ref(), id.into_inner(), "pago").await
}

#[get("/cuotas")]
async fn listar_cuotas(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
    let filas = cambio_cuota::Entity::find()
        .order_by_asc(cambio_cuota::Column::Id)
        .all(db.get_ref())
        .await
        .map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(filas))
}

#[get("/ingresos")]
async fn listar_ingresos(db: web::Data<DatabaseConnection>) -> Result<HttpResponse, ApiError> {
    let filas = db::ingresos(db.get_ref()).await.map_err(ApiError::db)?;
    Ok(HttpResponse::Ok().json(filas))
}

#[derive(Deserialize)]
struct ConId {
    id: i32,
}

#[derive(Deserialize)]
struct CambioAlumnoDireccion {
    id: i32,
    nombre: Option<String>,
    cartera_ref: Option<String>,
}

#[derive(Deserialize)]
struct CambioTalleristaDireccion {
    id: i32,
    nombre: Option<String>,
    oficio: Option<String>,
    pey_api_key_id: Option<String>,
}

#[derive(Deserialize)]
struct CuotaDireccion {
    id: i32,
    cuota_mensual_mxn: i32,
}

#[derive(Deserialize)]
struct CambioInscripcionDireccion {
    id: i32,
    activa: i32,
}

#[derive(Deserialize)]
struct CambioPagoDireccion {
    id: i32,
    estado: Option<String>,
    monto_mxn: Option<i32>,
}

#[get("/alumnos/agregar")]
async fn alumno_agregar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<NuevoAlumno>,
) -> Result<HttpResponse, ApiError> {
    guardar_alumno(db, web::Json(query.into_inner())).await
}

#[get("/alumnos/cambiar")]
async fn alumno_cambiar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<CambioAlumnoDireccion>,
) -> Result<HttpResponse, ApiError> {
    let datos = query.into_inner();
    guardar_cambio_alumno(
        db,
        web::Path::from(datos.id),
        web::Json(CambioAlumno {
            nombre: datos.nombre,
            cartera_ref: datos.cartera_ref,
        }),
    )
    .await
}

#[get("/alumnos/borrar")]
async fn alumno_borrar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
    borrar_en_navegador::<alumno::Entity>(db.get_ref(), query.id, "alumno").await
}

#[get("/talleristas/agregar")]
async fn tallerista_agregar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<NuevoTallerista>,
) -> Result<HttpResponse, ApiError> {
    guardar_tallerista(db, web::Json(query.into_inner())).await
}

#[get("/talleristas/cambiar")]
async fn tallerista_cambiar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<CambioTalleristaDireccion>,
) -> Result<HttpResponse, ApiError> {
    let datos = query.into_inner();
    guardar_cambio_tallerista(
        db,
        web::Path::from(datos.id),
        web::Json(CambioTallerista {
            nombre: datos.nombre,
            oficio: datos.oficio,
            pey_api_key_id: datos.pey_api_key_id,
        }),
    )
    .await
}

#[get("/talleristas/borrar")]
async fn tallerista_borrar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
    borrar_en_navegador::<tallerista::Entity>(db.get_ref(), query.id, "tallerista").await
}

#[get("/talleres/agregar")]
async fn taller_agregar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<NuevoTaller>,
) -> Result<HttpResponse, ApiError> {
    guardar_taller(db, web::Json(query.into_inner())).await
}

#[get("/talleres/cuota")]
async fn taller_cuota_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<CuotaDireccion>,
) -> Result<HttpResponse, ApiError> {
    let datos = query.into_inner();
    guardar_cuota(
        db,
        web::Path::from(datos.id),
        web::Json(NuevaCuota {
            cuota_mensual_mxn: datos.cuota_mensual_mxn,
        }),
    )
    .await
}

#[get("/talleres/borrar")]
async fn taller_borrar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
    borrar_en_navegador::<taller::Entity>(db.get_ref(), query.id, "taller").await
}

#[get("/nfc/agregar")]
async fn nfc_agregar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<NuevoNfc>,
) -> Result<HttpResponse, ApiError> {
    guardar_nfc(db, web::Json(query.into_inner())).await
}

#[get("/nfc/borrar")]
async fn nfc_borrar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
    borrar_en_navegador::<nfc::Entity>(db.get_ref(), query.id, "nfc").await
}

#[get("/anuales/agregar")]
async fn anual_agregar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<NuevaAnual>,
) -> Result<HttpResponse, ApiError> {
    guardar_anual(db, web::Json(query.into_inner())).await
}

#[get("/anuales/borrar")]
async fn anual_borrar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
    borrar_en_navegador::<anual::Entity>(db.get_ref(), query.id, "inscripcion anual").await
}

#[get("/inscripciones/agregar")]
async fn inscripcion_agregar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<NuevaInscripcion>,
) -> Result<HttpResponse, ApiError> {
    guardar_inscripcion(db, web::Json(query.into_inner())).await
}

#[get("/inscripciones/cambiar")]
async fn inscripcion_cambiar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<CambioInscripcionDireccion>,
) -> Result<HttpResponse, ApiError> {
    let datos = query.into_inner();
    guardar_cambio_inscripcion(
        db,
        web::Path::from(datos.id),
        web::Json(CambioInscripcion { activa: datos.activa }),
    )
    .await
}

#[get("/inscripciones/borrar")]
async fn inscripcion_borrar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
    borrar_en_navegador::<inscripcion::Entity>(db.get_ref(), query.id, "inscripcion").await
}

#[get("/pagos/agregar")]
async fn pago_agregar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<NuevoPago>,
) -> Result<HttpResponse, ApiError> {
    guardar_pago(db, web::Json(query.into_inner())).await
}

#[get("/pagos/cambiar")]
async fn pago_cambiar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<CambioPagoDireccion>,
) -> Result<HttpResponse, ApiError> {
    let datos = query.into_inner();
    guardar_cambio_pago(
        db,
        web::Path::from(datos.id),
        web::Json(CambioPago {
            estado: datos.estado,
            monto_mxn: datos.monto_mxn,
        }),
    )
    .await
}

#[get("/pagos/borrar")]
async fn pago_borrar_navegador(
    db: web::Data<DatabaseConnection>,
    query: web::Query<ConId>,
) -> Result<HttpResponse, ApiError> {
    borrar_en_navegador::<pago::Entity>(db.get_ref(), query.id, "pago").await
}

async fn borrar_en_navegador<E>(
    db: &DatabaseConnection,
    id: i32,
    nombre: &str,
) -> Result<HttpResponse, ApiError>
where
    E: EntityTrait,
    E::PrimaryKey: sea_orm::PrimaryKeyTrait<ValueType = i32>,
{
    eliminar::<E>(db, id, nombre).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "borrado": nombre,
        "id": id
    })))
}

async fn eliminar<E>(
    db: &DatabaseConnection,
    id: i32,
    nombre: &str,
) -> Result<HttpResponse, ApiError>
where
    E: EntityTrait,
    E::PrimaryKey: sea_orm::PrimaryKeyTrait<ValueType = i32>,
{
    let resultado = E::delete_by_id(id).exec(db).await.map_err(ApiError::db)?;
    if resultado.rows_affected == 0 {
        return Err(ApiError::no(nombre, id));
    }
    Ok(HttpResponse::NoContent().finish())
}

fn exigir_texto(valor: &str, campo: &str) -> Result<(), ApiError> {
    if valor.trim().is_empty() {
        Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            format!("{campo} no puede estar vacío"),
        ))
    } else {
        Ok(())
    }
}

fn exigir_monto(monto: i32) -> Result<(), ApiError> {
    if monto <= 0 {
        Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "el monto debe ser mayor que 0",
        ))
    } else {
        Ok(())
    }
}

fn exigir_cara(cara: &str) -> Result<(), ApiError> {
    if cara == "cobro" || cara == "inscripcion" {
        Ok(())
    } else {
        Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "la cara debe ser cobro o inscripcion",
        ))
    }
}

fn exigir_tipo(tipo: &str) -> Result<(), ApiError> {
    if tipo == "mensualidad" || tipo == "inscripcion_anual" {
        Ok(())
    } else {
        Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "el tipo debe ser mensualidad o inscripcion_anual",
        ))
    }
}

fn exigir_canal(canal: &str) -> Result<(), ApiError> {
    if canal == "boton" || canal == "nfc" || canal == "panel" {
        Ok(())
    } else {
        Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "el canal debe ser boton, nfc o panel",
        ))
    }
}

fn exigir_estado(estado: &str) -> Result<(), ApiError> {
    if estado == "pendiente" || estado == "pagado" || estado == "fallido" {
        Ok(())
    } else {
        Err(ApiError::mensaje(
            StatusCode::BAD_REQUEST,
            "el estado debe ser pendiente, pagado o fallido",
        ))
    }
}

fn marca_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duracion| duracion.as_millis())
        .unwrap_or(0)
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn mensaje(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn no(nombre: &str, id: i32) -> Self {
        Self::mensaje(
            StatusCode::NOT_FOUND,
            format!("No hay {nombre} con id {id}"),
        )
    }

    fn db(error: DbErr) -> Self {
        let texto = error.to_string();
        let (status, message) = if texto.contains("FOREIGN KEY") {
            (
                StatusCode::BAD_REQUEST,
                "Ese id no existe, o el registro todavía tiene inscripciones, pagos o talleres ligados.".to_string(),
            )
        } else if texto.contains("UNIQUE") {
            (
                StatusCode::BAD_REQUEST,
                "Ya existe un registro con ese dato.".to_string(),
            )
        } else if texto.contains("CHECK") {
            (
                StatusCode::BAD_REQUEST,
                "Algún dato no cumple las reglas de monto, canal, estado o tipo.".to_string(),
            )
        } else if let Some((_, aviso)) = texto.split_once("aviso: ") {
            (StatusCode::BAD_REQUEST, aviso.to_string())
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, texto)
        };
        Self { status, message }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).json(serde_json::json!({ "error": self.message }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};

    macro_rules! abrir_sesion {
        ($servicio:expr, $usuario:expr, $clave:expr) => {{
            let entrada = test::call_service(
                $servicio,
                test::TestRequest::get()
                    .uri(&format!(
                        "/cuenta/entrar?usuario={}&clave={}",
                        $usuario, $clave
                    ))
                    .to_request(),
            )
            .await;
            assert!(entrada.status().is_success());
            galleta_de(&entrada)
        }};
    }

    #[actix_web::test]
    async fn la_pagina_guia_la_cuenta_nueva() {
        let path = std::env::temp_dir().join(format!(
            "cuentas-pagina-{}-{}.db",
            std::process::id(),
            marca_ms()
        ));
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let db = db::conectar(&url).await.expect("base de prueba");
        let servicio = test::init_service(
            App::new()
                .app_data(web::Data::new(db))
                .wrap(crate::auth::ControlAcceso)
                .configure(config),
        )
        .await;

        let portada = test::call_service(
            &servicio,
            test::TestRequest::get().uri("/").to_request(),
        )
        .await;
        assert!(portada.status().is_success());
        let texto = test::read_body(portada).await;
        let texto = String::from_utf8(texto.to_vec()).unwrap();
        assert!(texto.contains("Iniciar sesión"));
        assert!(texto.contains("Crear una cuenta"));

        #[derive(serde::Serialize)]
        struct Forma {
            usuario: &'static str,
            clave: &'static str,
        }
        let alta = test::call_service(
            &servicio,
            test::TestRequest::post()
                .uri("/crear")
                .set_form(Forma {
                    usuario: "nila",
                    clave: "ceramica-2026",
                })
                .to_request(),
        )
        .await;
        assert_eq!(alta.status(), StatusCode::SEE_OTHER);
        let sesion = galleta_de(&alta);

        let paso = test::call_service(
            &servicio,
            con_sesion(
                &sesion,
                test::TestRequest::get()
                    .uri("/cuenta")
                    .insert_header((actix_web::http::header::ACCEPT, "text/html")),
            )
            .to_request(),
        )
        .await;
        assert!(paso.status().is_success());
        let texto = String::from_utf8(test::read_body(paso).await.to_vec()).unwrap();
        assert!(texto.contains("Tu nombre y tu oficio"));

        #[derive(serde::Serialize)]
        struct Datos {
            nombre: &'static str,
            oficio: &'static str,
        }
        let datos = test::call_service(
            &servicio,
            con_sesion(
                &sesion,
                test::TestRequest::post()
                    .uri("/cuenta/datos")
                    .set_form(Datos {
                        nombre: "Nila",
                        oficio: "ceramica",
                    }),
            )
            .to_request(),
        )
        .await;
        assert_eq!(datos.status(), StatusCode::SEE_OTHER);
        let _ = std::fs::remove_file(path);
    }

    #[actix_web::test]
    async fn el_servidor_lista_y_modifica_alumnos() {
        let path = std::env::temp_dir().join(format!(
            "cuentas-test-{}-{}.db",
            std::process::id(),
            marca_ms()
        ));
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let db = db::conectar(&url).await.expect("base de prueba");
        let servicio = test::init_service(
            App::new()
                .app_data(web::Data::new(db))
                .wrap(crate::auth::ControlAcceso)
                .configure(config),
        )
        .await;

        let sin_sesion = test::call_service(
            &servicio,
            test::TestRequest::get().uri("/resumen").to_request(),
        )
        .await;
        assert_eq!(sin_sesion.status(), StatusCode::UNAUTHORIZED);

        let respuesta = test::call_service(
            &servicio,
            admin(test::TestRequest::get().uri("/resumen")).to_request(),
        )
        .await;
        assert!(respuesta.status().is_success());
        let cuerpo: serde_json::Value = test::read_body_json(respuesta).await;
        assert_eq!(cuerpo["alumnos"], 10);
        assert_eq!(cuerpo["inscripcion_anual_mxn"], 125);

        let alta = test::call_service(
            &servicio,
            admin(test::TestRequest::post().uri("/alumnos").set_json(serde_json::json!({
                "nombre": "Lucía Méndez",
                "cartera_ref": "cartera_lucia"
            })))
            .to_request(),
        )
        .await;
        assert_eq!(alta.status(), StatusCode::CREATED);
        let creado: serde_json::Value = test::read_body_json(alta).await;
        let id = creado["id"].as_i64().unwrap();

        let baja = test::call_service(
            &servicio,
            admin(test::TestRequest::delete().uri(&format!("/alumnos/{id}"))).to_request(),
        )
        .await;
        assert_eq!(baja.status(), StatusCode::NO_CONTENT);

        let bloqueado = test::call_service(
            &servicio,
            admin(test::TestRequest::delete().uri("/alumnos/1")).to_request(),
        )
        .await;
        assert_eq!(bloqueado.status(), StatusCode::BAD_REQUEST);
        let _ = std::fs::remove_file(path);
    }

    #[actix_web::test]
    async fn la_cuenta_muestra_solo_los_datos_del_tallerista() {
        let path = std::env::temp_dir().join(format!(
            "cuentas-cuenta-{}-{}.db",
            std::process::id(),
            marca_ms()
        ));
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let db = db::conectar(&url).await.expect("base de prueba");
        let servicio = test::init_service(
            App::new()
                .app_data(web::Data::new(db))
                .wrap(crate::auth::ControlAcceso)
                .configure(config),
        )
        .await;

        let cerrada = test::call_service(
            &servicio,
            test::TestRequest::get().uri("/cuenta").to_request(),
        )
        .await;
        let icono = test::call_service(
            &servicio,
            test::TestRequest::get().uri("/favicon.ico").to_request(),
        )
        .await;
        assert_eq!(icono.status(), StatusCode::NOT_FOUND);
        assert!(
            icono
                .headers()
                .get(actix_web::http::header::WWW_AUTHENTICATE)
                .is_none()
        );

        assert_eq!(cerrada.status(), StatusCode::UNAUTHORIZED);
        assert!(
            cerrada
                .headers()
                .get(actix_web::http::header::WWW_AUTHENTICATE)
                .is_none()
        );

        let sesion_marichuy = abrir_sesion!(&servicio, "marichuy", "telar-2026");
        let sesion_pepe = abrir_sesion!(&servicio, "pepe", "color-2026");

        let ajena = test::call_service(
            &servicio,
            con_sesion(&sesion_marichuy, test::TestRequest::get().uri("/alumnos")).to_request(),
        )
        .await;
        assert_eq!(ajena.status(), StatusCode::UNAUTHORIZED);

        let respuesta = test::call_service(
            &servicio,
            con_sesion(&sesion_marichuy, test::TestRequest::get().uri("/cuenta")).to_request(),
        )
        .await;
        assert!(respuesta.status().is_success());
        let cuerpo: serde_json::Value = test::read_body_json(respuesta).await;
        assert_eq!(cuerpo["tallerista"], "Marichuy");
        assert_eq!(cuerpo["usuario"], "marichuy");
        let talleres = cuerpo["talleres"].as_array().unwrap();
        assert_eq!(talleres.len(), 1);
        assert_eq!(talleres[0]["nombre"], "Tejido");
        let dias: Vec<&str> = talleres[0]["horarios"]
            .as_array()
            .unwrap()
            .iter()
            .map(|horario| horario["dia"].as_str().unwrap())
            .collect();
        assert_eq!(dias, ["martes", "jueves"]);
        let alumnos = talleres[0]["alumnos"].as_array().unwrap();
        let camila = alumnos
            .iter()
            .find(|alumno| alumno["nombre"] == "Camila Flores")
            .unwrap();
        assert_eq!(camila["mensualidad"], "pendiente");
        let ana = alumnos
            .iter()
            .find(|alumno| alumno["nombre"] == "Ana López")
            .unwrap();
        assert_eq!(ana["mensualidad"], "pagado");

        let pepe = test::call_service(
            &servicio,
            con_sesion(&sesion_pepe, test::TestRequest::get().uri("/cuenta")).to_request(),
        )
        .await;
        let cuerpo: serde_json::Value = test::read_body_json(pepe).await;
        let nombres: Vec<&str> = cuerpo["talleres"][0]["alumnos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|alumno| alumno["nombre"].as_str().unwrap())
            .collect();
        assert!(nombres.contains(&"Luis Hernández"));
        assert!(!nombres.contains(&"Camila Flores"));
        let _ = std::fs::remove_file(path);
    }

    #[actix_web::test]
    async fn un_tallerista_nuevo_crea_su_cuenta_y_la_completa() {
        let path = std::env::temp_dir().join(format!(
            "cuentas-nueva-{}-{}.db",
            std::process::id(),
            marca_ms()
        ));
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let db = db::conectar(&url).await.expect("base de prueba");
        let servicio = test::init_service(
            App::new()
                .app_data(web::Data::new(db))
                .wrap(crate::auth::ControlAcceso)
                .configure(config),
        )
        .await;

        let corta = test::call_service(
            &servicio,
            test::TestRequest::get()
                .uri("/cuenta/crear?usuario=rosa&clave=123")
                .to_request(),
        )
        .await;
        assert_eq!(corta.status(), StatusCode::BAD_REQUEST);

        let alta = test::call_service(
            &servicio,
            test::TestRequest::get()
                .uri("/cuenta/crear?usuario=rosa&clave=ceramica-2026")
                .to_request(),
        )
        .await;
        assert_eq!(alta.status(), StatusCode::CREATED);
        assert!(
            alta.headers()
                .get(actix_web::http::header::SET_COOKIE)
                .is_some()
        );
        let creada: serde_json::Value = test::read_body_json(alta).await;
        assert_eq!(creada["usuario"], "rosa");
        assert!(creada.get("clave").is_none());

        let repetida = test::call_service(
            &servicio,
            test::TestRequest::get()
                .uri("/cuenta/crear?usuario=rosa&clave=ceramica-2026")
                .to_request(),
        )
        .await;
        assert_eq!(repetida.status(), StatusCode::BAD_REQUEST);

        let mal = test::call_service(
            &servicio,
            test::TestRequest::get()
                .uri("/cuenta/entrar?usuario=rosa&clave=otra-clave")
                .to_request(),
        )
        .await;
        assert_eq!(mal.status(), StatusCode::BAD_REQUEST);

        let entrada = test::call_service(
            &servicio,
            test::TestRequest::get()
                .uri("/cuenta/entrar?usuario=rosa&clave=ceramica-2026")
                .to_request(),
        )
        .await;
        assert!(entrada.status().is_success());
        let sesion_rosa = galleta_de(&entrada);

        let vacia = test::call_service(
            &servicio,
            con_sesion(&sesion_rosa, test::TestRequest::get().uri("/cuenta")).to_request(),
        )
        .await;
        let cuerpo: serde_json::Value = test::read_body_json(vacia).await;
        assert_eq!(cuerpo["listo"], false);
        assert!(cuerpo["oficio"].as_str().unwrap().is_empty());

        let datos = test::call_service(
            &servicio,
            con_sesion(
                &sesion_rosa,
                test::TestRequest::get().uri("/cuenta/datos?nombre=Rosa&oficio=ceramica"),
            )
            .to_request(),
        )
        .await;
        assert!(datos.status().is_success());

        let taller = test::call_service(
            &servicio,
            con_sesion(
                &sesion_rosa,
                test::TestRequest::get()
                    .uri("/cuenta/taller?nombre=Ceramica&cuota_mensual_mxn=30"),
            )
            .to_request(),
        )
        .await;
        assert!(taller.status().is_success());

        let horario = test::call_service(
            &servicio,
            con_sesion(
                &sesion_rosa,
                test::TestRequest::get().uri("/cuenta/horario?dia=martes&de=16:00&a=18:00"),
            )
            .to_request(),
        )
        .await;
        assert!(horario.status().is_success());

        let alumno = test::call_service(
            &servicio,
            con_sesion(
                &sesion_rosa,
                test::TestRequest::get().uri("/cuenta/alumno?nombre=Nila%20Prueba"),
            )
            .to_request(),
        )
        .await;
        assert!(alumno.status().is_success());
        let cuerpo: serde_json::Value = test::read_body_json(alumno).await;
        assert_eq!(cuerpo["listo"], true);
        assert_eq!(cuerpo["tallerista"], "Rosa");
        assert_eq!(cuerpo["oficio"], "ceramica");
        assert_eq!(cuerpo["talleres"][0]["nombre"], "Ceramica");
        assert_eq!(cuerpo["talleres"][0]["horarios"][0]["dia"], "martes");
        assert_eq!(cuerpo["talleres"][0]["alumnos"][0]["nombre"], "Nila Prueba");
        assert_eq!(cuerpo["talleres"][0]["alumnos"][0]["mensualidad"], "pendiente");

        let pago = test::call_service(
            &servicio,
            con_sesion(
                &sesion_rosa,
                test::TestRequest::get().uri("/cuenta/pago?alumno=Nila%20Prueba&estado=pagado"),
            )
            .to_request(),
        )
        .await;
        let cuerpo: serde_json::Value = test::read_body_json(pago).await;
        assert_eq!(cuerpo["talleres"][0]["alumnos"][0]["mensualidad"], "pagado");

        let sesion_marichuy = abrir_sesion!(&servicio, "marichuy", "telar-2026");
        let ajena = test::call_service(
            &servicio,
            con_sesion(&sesion_marichuy, test::TestRequest::get().uri("/cuenta")).to_request(),
        )
        .await;
        let cuerpo: serde_json::Value = test::read_body_json(ajena).await;
        assert_eq!(cuerpo["talleres"][0]["nombre"], "Tejido");
        let _ = std::fs::remove_file(path);
    }

    fn admin(peticion: actix_web::test::TestRequest) -> actix_web::test::TestRequest {
        tallerista(db::ADMIN_USUARIO, db::ADMIN_CLAVE, peticion)
    }

    fn tallerista(
        usuario: &str,
        clave: &str,
        peticion: actix_web::test::TestRequest,
    ) -> actix_web::test::TestRequest {
        peticion.insert_header((
            actix_web::http::header::AUTHORIZATION,
            crate::auth::encabezado_basic(usuario, clave),
        ))
    }

    fn con_sesion(
        sesion: &str,
        peticion: actix_web::test::TestRequest,
    ) -> actix_web::test::TestRequest {
        peticion.insert_header((actix_web::http::header::COOKIE, sesion))
    }

    fn galleta_de(respuesta: &actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>) -> String {
        respuesta
            .headers()
            .get(actix_web::http::header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }
}
