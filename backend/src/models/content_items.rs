//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "content_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub content_type_id: i32,
    #[sea_orm(column_type = "JsonBinary")]
    pub data: Json,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::content_types::Entity",
        from = "Column::ContentTypeId",
        to = "super::content_types::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    ContentTypes,
}

impl Related<super::content_types::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ContentTypes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
