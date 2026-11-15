use sea_orm::entity::prelude::*;
use serde::Serialize;

pub mod alumno {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "alumnos")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub nombre: String,
        pub cartera_ref: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod tallerista {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "talleristas")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub nombre: String,
        pub oficio: String,
        pub pey_api_key_id: String,
        pub usuario: Option<String>,
        pub telefono: Option<String>,
        #[serde(skip_serializing)]
        pub clave_hash: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod taller {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "talleres")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub tallerista_id: i32,
        pub nombre: String,
        pub cuota_mensual_mxn: i32,
        pub activo: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod cambio_cuota {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "cambios_cuota")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub taller_id: i32,
        pub cuota_anterior_mxn: Option<i32>,
        pub cuota_nueva_mxn: i32,
        pub vigente_desde: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod nfc {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "dispositivos_nfc")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub tallerista_id: i32,
        pub cara: String,
        pub chip_uid: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod anual {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "inscripciones_anuales")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub alumno_id: i32,
        pub anio: i32,
        pub monto_mxn: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod inscripcion {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "inscripciones_taller")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub alumno_id: i32,
        pub taller_id: i32,
        pub anio: i32,
        pub activa: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod pago {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
    #[sea_orm(table_name = "pagos")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub alumno_id: i32,
        pub taller_id: Option<i32>,
        pub tallerista_id: Option<i32>,
        pub tipo: String,
        pub canal: String,
        pub concepto: String,
        pub monto_mxn: i32,
        pub num_ref: String,
        pub codi_qr_id: Option<String>,
        pub transaction_id: Option<String>,
        pub tipo_evento: Option<String>,
        pub estado: String,
        pub periodo: String,
        pub pagado_en: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
