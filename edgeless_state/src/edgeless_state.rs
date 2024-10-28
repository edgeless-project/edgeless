use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool, Row, FromRow};
use tokio;
const DB_URL: &str = "sqlite://sqlite.db";

#[tokio::main]
async fn main() {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }
    
    //connect to the db
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let result = sqlx::query("CREATE TABLE IF NOT EXISTS workflow (
        id INTEGER PRIMARY KEY,
        name VARCHAR(255)  NOT NULL,
        result INTEGER NOT NULL);").execute(&db).await.unwrap();
    println!("Create workflow table result: {:?}", result);
    
    //insert something //need to solve if the result exist
    let result = sqlx::query("INSERT INTO workflow (id, name, result) Values($1, $2, $3)")
        .bind("5")
        .bind("smart suvelliance")
        .bind("1000")
        .execute(&db)
        .await
        .unwrap();
    println!("Query result: {:?}", result);

    //query all items in the table
    let query_results = sqlx::query_as::<_, Workflow>("SELECT id, name, result FROM workflow")
        .fetch_all(&db)
        .await
        .unwrap();
    for workflow in query_results {
        println!("[{}] name: {}, result {}", workflow.id, &workflow.name, workflow.result);
    }

    //delete result
    let delete_result = sqlx::query("DELETE FROM workflow WHERE name=$1")
        .bind("smart suvelliance")
        .execute(&db)
        .await
        .unwrap();
    println!("Delete result: {:?}", delete_result);

    //update
}

#[derive(Clone, FromRow, Debug)]
struct Workflow {
    id: i64,
    name: String,
    result: i64,
}
