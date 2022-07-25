use std::env;

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, query::Query, Pool, Postgres};

mod queries;

// async fn init_postgress() -> anyhow::Result<()> {
//     // let hostname = env::var("POSTGRESS_HOST")?;
//     // let user = env::var("POSTGRESS_USER")?;
//     // let password = env::var("POSTGRESS_PASSWORD")?;
//     // let connection_string = format!("host={} user={} password={}", hostname, user, password);

//     // let (client, conn) = tokio_postgres::connect(&connection_string, tokio_postgres::NoTls).await?;

//     // tokio::spawn(async move {
//     //     if let Err(e) = conn.await {
//     //         eprintln!("postgress connection error: {}", e);
//     //     }
//     // });

//     // Ok(())
// }
struct Foo {
    foo: i64,
}

///Trait abstracting storage requirements for DDJ
#[rocket::async_trait]
pub trait PersistentStore {
    async fn create_tables(&self) -> Result<()>;
}

///Literally a Box<dyn PersistentStore + Send + Sync>
/// Using this type allows the database implementation to be
/// swapped at runtime
pub type Store = Box<dyn PersistentStore + Send + Sync>;

pub struct PostgressDatabase {
    pool: Pool<Postgres>,
}

impl PostgressDatabase {
    pub async fn connect() -> Result<Store> {
        let hostname = env::var("POSTGRESS_HOST")?;
        let user = env::var("POSTGRESS_USER")?;
        let password = env::var("POSTGRESS_PASSWORD")?;

        let connection_str = format!("postgres://{user}:{password}@{hostname}/ddj");

        let conn_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_str)
            .await?;

        Ok(Box::new(Self { pool: conn_pool }))
    }
}

#[rocket::async_trait]
impl PersistentStore for PostgressDatabase {
    async fn create_tables(&self) -> Result<()> {
        sqlx::query(queries::CREATE_TASK_TABLE)
            .execute(&self.pool)
            .await?;

        sqlx::query(queries::CREATE_PLAYED_TRACKS_TABLE)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
