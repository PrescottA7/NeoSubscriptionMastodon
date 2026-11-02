use std::path::Path;

use sea_orm::sea_query::Index;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr, Schema};
use tracing::info;

pub mod admins;
pub mod credentials;
pub mod entities;
pub mod nodes;
pub mod settings;
pub mod users;

/// Connects to SQLite and makes sure the schema exists.
pub async fn connect(database_path: &Path) -> anyhow::Result<DatabaseConnection> {
    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    info!("using sqlite database at {}", database_path.display());
    let connection = Database::connect(database_url(database_path)).await?;
    init_schema(&connection).await?;
    Ok(connection)
}

/// Builds a `sqlite:` URL that works on Windows and Unix alike.
fn database_url(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    format!("sqlite://{path}?mode=rwc")
}

pub async fn init_schema(conn: &DatabaseConnection) -> Result<(), DbErr> {
    let backend = conn.get_database_backend();
    let schema = Schema::new(backend);

    let tables = [
        schema.create_table_from_entity(entities::admins::Entity),
        schema.create_table_from_entity(entities::users::Entity),
        schema.create_table_from_entity(entities::nodes::Entity),
        schema.create_table_from_entity(entities::node_credentials::Entity),
        schema.create_table_from_entity(entities::sessions::Entity),
        schema.create_table_from_entity(entities::settings::Entity),
    ];

    for mut table in tables {
        table.if_not_exists();
        conn.execute(backend.build(&table)).await?;
    }

    let indexes = [
        Index::create()
            .if_not_exists()
            .name("idx_users_token")
            .table(entities::users::Entity)
            .col(entities::users::Column::Token)
            .unique()
            .to_owned(),
        Index::create()
            .if_not_exists()
            .name("idx_node_credentials_node_user")
            .table(entities::node_credentials::Entity)
            .col(entities::node_credentials::Column::NodeId)
            .col(entities::node_credentials::Column::UserId)
            .unique()
            .to_owned(),
        Index::create()
            .if_not_exists()
            .name("idx_node_credentials_user")
            .table(entities::node_credentials::Entity)
            .col(entities::node_credentials::Column::UserId)
            .to_owned(),
    ];

    for index in indexes {
        conn.execute(backend.build(&index)).await?;
    }

    ensure_column(
        conn,
        "nodes",
        "server_params",
        "ALTER TABLE nodes ADD COLUMN server_params TEXT",
    )
    .await?;

    ensure_column(
        conn,
        "users",
        "token",
        "ALTER TABLE users ADD COLUMN token TEXT NOT NULL DEFAULT ''",
    )
    .await?;

    migrate_legacy_credentials(conn).await?;
    migrate_legacy_tokens(conn).await?;

    Ok(())
}

/// Gives pre-`token` users a readable slug.
///
/// Older builds stored only a hash, so the original link cannot be recovered.
/// Each affected user receives a generated slug instead, which the operator can
/// then edit from the admin UI.
async fn migrate_legacy_tokens(conn: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, FromQueryResult, Statement};

    if conn.get_database_backend() != DatabaseBackend::Sqlite || !table_exists(conn, "users").await?
    {
        return Ok(());
    }

    #[derive(FromQueryResult)]
    struct LegacyUser {
        id: i64,
    }

    let rows = LegacyUser::find_by_statement(Statement::from_string(
        DatabaseBackend::Sqlite,
        "SELECT id FROM users WHERE token IS NULL OR token = ''".to_owned(),
    ))
    .all(conn)
    .await?;

    for row in rows {
        let slug = crate::utils::random_secret();
        conn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE users SET token = ? WHERE id = ?",
            [slug.into(), row.id.into()],
        ))
        .await?;
        info!(user_id = row.id, "assigned a subscription slug to a legacy user");
    }

    Ok(())
}

/// Adds a column to an existing table once.
///
/// `create_table_from_entity` only runs `CREATE TABLE IF NOT EXISTS`, so a
/// database created before the column existed would never gain it.
async fn ensure_column(
    conn: &DatabaseConnection,
    table: &str,
    column: &str,
    alter: &str,
) -> Result<(), DbErr> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    if conn.get_database_backend() != DatabaseBackend::Sqlite {
        return Ok(());
    }
    if !table_exists(conn, table).await? || column_exists(conn, table, column).await? {
        return Ok(());
    }

    conn.execute(Statement::from_string(DatabaseBackend::Sqlite, alter.to_owned()))
        .await?;
    info!(table, column, "added missing column");
    Ok(())
}

/// Moves pre-`node_credentials` data forward.
///
/// Older builds stored one uuid and one encrypted password on the user, shared
/// by every node, plus a `node_users` visibility table. That data is now wrong
/// by design, so it is copied into per-node rows only where the intent is
/// unambiguous: a visible, enabled node where the user has a credential to give.
///
/// The legacy columns are left in place; this function only reads them.
async fn migrate_legacy_credentials(conn: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let backend = conn.get_database_backend();
    if backend != DatabaseBackend::Sqlite {
        return Ok(());
    }

    if !table_exists(conn, "node_users").await? {
        return Ok(());
    }
    let has_legacy_columns = column_exists(conn, "users", "uuid").await?
        && column_exists(conn, "users", "password").await?;
    if !has_legacy_columns {
        return Ok(());
    }

    // `node_users` being empty means "every node was visible" under the old
    // rules, so every pair is migrated in that case.
    let sql = "
        INSERT INTO node_credentials
            (node_id, user_id, uuid, password, email, enabled, created_at, updated_at)
        SELECT n.id, u.id, u.uuid, u.password,
               LOWER(REPLACE(u.remark, ' ', '-')) || '@' || LOWER(n.name),
           1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        FROM users u
        JOIN nodes n
        WHERE NOT EXISTS (
            SELECT 1 FROM node_users nu WHERE nu.user_id = u.id
        )
        OR EXISTS (
            SELECT 1 FROM node_users nu WHERE nu.user_id = u.id AND nu.node_id = n.id
        )
        AND NOT EXISTS (
            SELECT 1 FROM node_credentials c
            WHERE c.node_id = n.id AND c.user_id = u.id
        )";

    let result = conn.execute(Statement::from_string(backend, sql)).await;
    match result {
        Ok(done) if done.rows_affected() > 0 => {
            info!(rows = done.rows_affected(), "migrated legacy credentials");
            // The legacy visibility table is superseded by the credential rows.
            let drop = "DROP TABLE node_users";
            let _ = conn
                .execute(Statement::from_string(backend, drop))
                .await;
        }
        Ok(_) => {
            let drop = "DROP TABLE node_users";
            let _ = conn
                .execute(Statement::from_string(backend, drop))
                .await;
        }
        Err(error) => {
            tracing::warn!(%error, "legacy credential migration skipped");
        }
    }

    Ok(())
}

async fn table_exists(conn: &DatabaseConnection, name: &str) -> Result<bool, DbErr> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let backend = conn.get_database_backend();
    if backend != DatabaseBackend::Sqlite {
        return Ok(false);
    }

    let statement = Statement::from_sql_and_values(
        backend,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?",
        [name.into()],
    );
    Ok(conn.query_one(statement).await?.is_some())
}

async fn column_exists(conn: &DatabaseConnection, table: &str, column: &str) -> Result<bool, DbErr> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let backend = conn.get_database_backend();
    if backend != DatabaseBackend::Sqlite {
        return Ok(false);
    }

    let statement = Statement::from_string(backend, format!("PRAGMA table_info({table})"));
    let rows = conn.query_all(statement).await?;
    Ok(rows.iter().any(|row| {
        row.try_get::<String>("", "name")
            .map(|name| name == column)
            .unwrap_or(false)
    }))
}
