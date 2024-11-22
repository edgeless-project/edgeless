// use sqlx::{migrate::MigrateDatabase, FromRow, Row, Sqlite, SqlitePool};
use sqlx::{migrate::MigrateDatabase, FromRow, Sqlite, SqlitePool};
use tokio;
const DB_URL: &str = "sqlite://sqlite.db";

// #[tokio::main]
// async fn main() {
//     if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
//         println!("Creating database {}", DB_URL);
//         match Sqlite::create_database(DB_URL).await {
//             Ok(_) => println!("Create db success"),
//             Err(error) => panic!("error: {}", error),
//         }
//     } else {
//         println!("Database already exists");
//     }

//     //connect to the db
//     let db = SqlitePool::connect(DB_URL).await.unwrap();
//     let result = sqlx::query(
//         "CREATE TABLE IF NOT EXISTS workflow (
//         id INTEGER PRIMARY KEY,
//         name VARCHAR(255)  NOT NULL,
//         result INTEGER NOT NULL);",
//     )
//     .execute(&db)
//     .await
//     .unwrap();
//     println!("Create workflow table result: {:?}", result);

//     //insert something //need to solve if the result exist
//     let result = sqlx::query("INSERT INTO workflow (id, name, result) Values($1, $2, $3)")
//         .bind("5")
//         .bind("smart suvelliance")
//         .bind("1000")
//         .execute(&db)
//         .await
//         .unwrap();
//     println!("Query result: {:?}", result);

//     //query all items in the table
//     let query_results = sqlx::query_as::<_, Workflow>("SELECT id, name, result FROM workflow")
//         .fetch_all(&db)
//         .await
//         .unwrap();
//     for workflow in query_results {
//         println!("[{}] name: {}, result {}", workflow.id, &workflow.name, workflow.result);
//     }

//     //delete result
//     let delete_result = sqlx::query("DELETE FROM workflow WHERE name=$1")
//         .bind("smart suvelliance")
//         .execute(&db)
//         .await
//         .unwrap();
//     println!("Delete result: {:?}", delete_result);

//     //update
// }

// #[tokio::main]
fn main() {
    create_db();
    // let db = SqlitePool::connect(DB_URL).await.unwrap();
    insert_state("999".to_string(), "foobar".to_string(), "007".to_string());
    // delete_state("999".to_string());
    get_state("999".to_string());
}

#[tokio::main]
async fn create_db() {
    println!("---create db---");
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }
}

//todo if it exists what to do
#[tokio::main]
async fn insert_state(id: String, name: String, result: String) {
    println!("---insert---");
    //connect to the db
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let response = sqlx::query(
        "CREATE TABLE IF NOT EXISTS workflow (
        id INTEGER PRIMARY KEY,
        name VARCHAR(255)  NOT NULL,
        result INTEGER NOT NULL);",
    )
    .execute(&db)
    .await
    .unwrap();
    println!("Create workflow table result: {:?}", response);

    //insert something //need to solve if the result exist
    let response = sqlx::query("INSERT INTO workflow (id, name, result) Values($1, $2, $3)")
        .bind(id)
        .bind(name)
        .bind(result)
        .execute(&db)
        .await;
    println!("Insert result: {:?}", response);
}

#[tokio::main]
async fn delete_state(id: String) {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    //delete result
    let delete_result = sqlx::query("DELETE FROM workflow WHERE id=$1").bind(id).execute(&db).await.unwrap();
    println!("Delete result: {:?}", delete_result);
}

//todo close connection everytime
#[tokio::main]
async fn get_state(id: String) {
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    //delete result
    // let ret = sqlx::query_as!(Workflow, "SELECT name,  result FROM workflow WHERE id=$1")
    //     .bind(id)
    //     .execute(&db)
    //     .await
    //     .unwrap();
    let ret: Workflow = sqlx::query_as("SELECT id, name,  result FROM workflow WHERE id=$1")
        .bind(id)
        .fetch_one(&db)
        .await
        .unwrap();
    println!("read result: {:?}", ret);
}

// fn update(id: String, attribute: String, value: String) {
//     let db = SqlitePool::connect(DB_URL).await.unwrap();
//     //update result
//     let update_result = sqlx::query("UPDATE workflow SET " + "$2=$3  WHERE id = $1")
//     .bind(id)
//     .bind(attribute)
//     .bind(value)
//     .execute(&db)
//     .await
//     .unwrap();
//     println!("update result: {:?}", update_result);

//     match update_result {
//         Ok(_) => println!("Update success"),
//         Err(error) => {
//             panic!("error: {}", error);
//         }
//     }
// }

#[derive(Clone, FromRow, Debug)]
struct Workflow {
    id: i64,
    name: String,
    result: i64,
}
