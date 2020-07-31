use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;

// 1. Use request handlers to extract parameters from a request(trait:FromRequest) and return a response(trait:Responder).
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

// 2. Use path utility to parse path parameters.
#[get("/{id}/{name}")]
async fn user(info: web::Path<(u32, String)>) -> impl Responder {
    HttpResponse::Ok().body(format!("Hello {}! id:{}", info.1, info.0))
}

// 3. Share state in the scope.
struct AppState {
    app_name: String,
    counter: Mutex<i32>,
}

#[get("")] // scope name will be the prefix of the path
async fn app1(data: web::Data<AppState>) -> impl Responder {
    let app_name = &data.app_name; // <- get app_name
    let mut counter = data.counter.lock().unwrap(); // <- get counter's MutexGuard
    *counter += 1;

    HttpResponse::Ok().body(format!(
        "Hello to {} and Request number is {}",
        app_name, counter
    ))
}

// 3. Use application configuration to setup handlers.
fn user_config(cfg: &mut web::ServiceConfig) {
    cfg.service(user);
}

// start point
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        app_name: String::from("hello-actix"),
        counter: Mutex::new(0),
    });

    // Use App instance to register routes, middlewares and to store state.
    HttpServer::new(move || {
        App::new()
            .service(index)
            .service(web::scope("/user").configure(user_config)) // Include the configuration.
            .service(web::scope("/app1").app_data(state.clone()).service(app1)) // Clone the state for each thread in the scope.
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
