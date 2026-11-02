use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::Serialize;

/// A subscription consumer.
///
/// The user owns a subscription slug and nothing else: proxy credentials are
/// per node and live in `node_credentials`, because each proxy server keeps its
/// own account list.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub remark: String,
    pub enabled: bool,
    /// The subscription slug: the user-visible, editable part of the link.
    pub token: String,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub access_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::node_credentials::Entity")]
    NodeCredentials,
}

impl Related<super::node_credentials::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NodeCredentials.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
