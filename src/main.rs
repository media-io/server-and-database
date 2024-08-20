mod post;
mod user;

use axum::extract::{Path, State};
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use migration::{Migrator, MigratorTrait};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, Database, DatabaseConnection, EntityTrait,
    QueryFilter,
};
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Port to serve
    #[arg(short, long, default_value_t = 4000)]
    port: u16,
    /// URL to the database
    #[arg(short, long, default_value = "sqlite::memory:")]
    database_url: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339(std::time::SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .level_for("hyper", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()
        .unwrap();

    let database_connection = Database::connect(args.database_url)
        .await
        .expect("Database connection failed");

    Migrator::up(&database_connection, None).await.unwrap();

    let app = Router::new()
        .route("/", get(root))
        .route("/users", get(list_users))
        .route("/users", post(create_user))
        .route("/users/:user_id/posts", get(list_posts))
        .route("/users/:user_id/posts", post(create_post))
        .layer(TraceLayer::new_for_http())
        .with_state(database_connection);

    let bind_address = format!("0.0.0.0:{}", args.port);

    let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
    println!("serve on {bind_address}");
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn list_users(state: State<DatabaseConnection>) -> (StatusCode, Json<Vec<User>>) {
    let users: Vec<user::Model> = user::Entity::find().all(&state.0).await.unwrap();

    let users = users
        .into_iter()
        .map(|model_user| User {
            id: model_user.id as u64,
            username: model_user.username,
        })
        .collect();

    (StatusCode::OK, Json(users))
}

async fn create_user(
    state: State<DatabaseConnection>,
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    let model_user = user::ActiveModel {
        username: Set(payload.username.to_string()),
        ..Default::default()
    };
    let model_user = model_user.insert(&state.0).await.unwrap();

    let user = User {
        id: model_user.id as u64,
        username: model_user.username,
    };

    (StatusCode::CREATED, Json(user))
}

async fn list_posts(
    state: State<DatabaseConnection>,
    Path(user_id): Path<u32>,
) -> (StatusCode, Json<Vec<Post>>) {
    let posts: Vec<post::Model> = post::Entity::find()
        .filter(Condition::any().add(post::Column::UserId.eq(user_id)))
        .all(&state.0)
        .await
        .unwrap();

    let posts = posts
        .into_iter()
        .map(|model_post| Post {
            id: model_post.id as u64,
            message: model_post.message,
        })
        .collect();

    (StatusCode::OK, Json(posts))
}

async fn create_post(
    state: State<DatabaseConnection>,
    Path(user_id): Path<u32>,
    Json(payload): Json<CreatePost>,
) -> (StatusCode, Json<Post>) {
    let model_post = post::ActiveModel {
        message: Set(payload.message.to_string()),
        user_id: Set(user_id as i32),
        ..Default::default()
    };
    let model_post = model_post.insert(&state.0).await.unwrap();

    let post = Post {
        id: model_post.id as u64,
        message: model_post.message,
    };

    (StatusCode::CREATED, Json(post))
}

#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}

#[derive(Deserialize)]
struct CreatePost {
    message: String,
}

#[derive(Serialize)]
struct Post {
    id: u64,
    message: String,
}
