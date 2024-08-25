use sqlx::{Pool, Sqlite, SqlitePool};
use  std::{fs,error::Error,path::Path,};

use ssl_observer_common::ProbeSslData;

use crate::decode::parse_http;
use crate::utils::{convert_timestamp_to_date, sanitize_comm};
use crate::config::CONFIG;

#[derive(sqlx::FromRow)]
pub struct SslDataRow {
    pub id: i64,
    pub timestamp: String,
    pub pid: i32,
    pub comm: String,
    pub buf: String,
}

pub async fn init_db() -> Result<SqlitePool, Box<dyn Error>> {
    let db_path= &CONFIG.database.sqlite_path();
    let database_name = CONFIG.database.sqlite_db_name();
    let create_table_query: String =format!(r#"CREATE TABLE IF NOT EXISTS {} (
        id INTEGER PRIMARY KEY AUTO_INCREMENT,
        timestamp TEXT,
        delta_ns INTEGER,
        comm TEXT,
        pid INTEGER,
        tgid INTEGER,
        uid INTEGER,
        buf_filled INTEGER,
        rw INTEGER,
        is_handshake INTEGER,
        len INTEGER,
        buf TEXT
    )"#,database_name);

    // 检查文件是否存在，如果不存在则创建
    if !Path::new(db_path).exists() {
        fs::File::create(db_path)?;
    }

    let pool: Pool<Sqlite> = SqlitePool::connect(db_path).await?;
    sqlx::query( &create_table_query).execute(&pool).await?;
    Ok(pool)
}

pub async fn query_data(pool: &SqlitePool) -> Result<Vec<SslDataRow>, Box<dyn Error>> {
    let select_table_query = format!("SELECT id, timestamp, pid, comm, buf FROM {} WHERE is_handshake = 0",&CONFIG.database.mysql_db_name()
);
    let rows: Vec<SslDataRow> = sqlx::query_as::<Sqlite, _>(
        &select_table_query,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}


pub async fn insert_data(pool: &SqlitePool, data: &ProbeSslData) -> Result<(), Box<dyn Error>> {
    let date: String = convert_timestamp_to_date(data.timestamp_ns).await?;
    let comm_cleaned: String = sanitize_comm(&data.comm);
    let content = parse_http(&data.buf[..data.len]).await;

    let insert_table_query = format!("INSERT INTO {} (timestamp, delta_ns, comm, pid, tgid, uid, buf_filled, rw, is_handshake, len, buf) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",CONFIG.database.sqlite_db_name());
    let _res = sqlx::query(&insert_table_query)
        .bind(date)
        .bind(data.delta_ns as i64)
        .bind(comm_cleaned)
        .bind(data.pid)
        .bind(data.tgid)
        .bind(data.uid)
        .bind(data.buf_filled)
        .bind(data.rw)
        .bind(data.is_handshake as i32)
        .bind(data.len as i32)
        .bind(content)
        .execute(pool)
        .await?;

    Ok(())
}