use actix_web::{get, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use futures::future::{ok, ready, Ready};
use futures::stream::once;

use serde::Serialize;

use bytes::Bytes;
use std::sync::Mutex;

// 1. Use request handlers to extract parameters from a request(trait:FromRequest) and return a response(trait:Responder).
// 2. By default actix-web provides Responder implementations for some standard types, such as &'static str, String, etc.
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

// 4. Use application configuration to setup handlers.
fn user_config(cfg: &mut web::ServiceConfig) {
    cfg.service(user);
}

// 5. Return custom object as response
#[derive(Serialize)]
struct MyObj {
    name: &'static str,
}

impl Responder for MyObj {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self).unwrap();

        // Create response and set content type
        ready(Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(body)))
    }
}

#[get("/custom")]
async fn custom() -> impl Responder {
    MyObj { name: "user" }
}

// 6. Return stream response
#[get("/stream")]
async fn stream() -> HttpResponse {
    let body = once(ok::<_, Error>(Bytes::from_static(b"stream")));

    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(body)
}

// start point
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        app_name: String::from("hello-actix"),
        counter: Mutex::new(0),
    });

    // Use App factory to register routes, middlewares and to store state. The shared data has to be thread-safe.
    HttpServer::new(move || {
        App::new()
            .service(index)
            .service(custom)
            .service(stream)
            .service(web::scope("/user").configure(user_config)) // Include the configuration.
            .service(web::scope("/app1").app_data(state.clone()).service(app1)) // Clone the state for each thread in the scope.
    })
    .bind("127.0.0.1:8080")?
    .run() // Run and return an instance of the server.
    .await
}
