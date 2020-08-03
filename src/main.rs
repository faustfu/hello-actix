use actix_http::ResponseBuilder;
use actix_web::http::{header, StatusCode};
use actix_web::{
    error, get, middleware::Logger, post, web, App, Error, HttpRequest, HttpResponse, HttpServer,
    Responder, Result,
};
use failure::Fail;
use futures::future::{ok, ready, Ready};
use futures::stream::once;

use serde::{Deserialize, Serialize};

use log::debug;

use bytes::Bytes;
use std::sync::Mutex;

// 1. Use request handlers to extract parameters from a request(trait:FromRequest) and return a response(trait:Responder).
// 2. By default actix-web provides Responder implementations for some standard types, such as &'static str, String, etc.
#[get("/")]
async fn index() -> HttpResponse {
    HttpResponse::Ok().body("Hey there!")
}

// 3. Use extractors to parse the request.
#[derive(Deserialize)]
struct Info {
    age: u32,
}
#[get("/{id}/{name}")]
async fn user_get(args: web::Path<(u32, String)>, info: web::Query<Info>) -> HttpResponse {
    HttpResponse::Ok().body(format!(
        "Hello {}! id:[{}], age:[{}]",
        args.1, args.0, info.age
    ))
}

#[derive(Deserialize)]
struct UserInfo {
    name: String,
}
#[post("")]
async fn user_post(info: web::Form<UserInfo>) -> HttpResponse {
    HttpResponse::Ok().body(format!("Hello {}!", info.name))
}

// 4. Share state in the scope.
struct AppState {
    app_name: String,
    counter: Mutex<i32>,
}

#[get("")] // scope name will be the prefix of the path
async fn app1(data: web::Data<AppState>) -> HttpResponse {
    let app_name = &data.app_name; // <- get app_name
    let mut counter = data.counter.lock().unwrap(); // <- get counter's MutexGuard
    *counter += 1;

    HttpResponse::Ok().body(format!(
        "Hello to {} and Request number is {}",
        app_name, counter
    ))
}

// 5. Use application configuration to setup handlers.
fn user_config(cfg: &mut web::ServiceConfig) {
    cfg.service(user_get).service(user_post);
}

// 6. Return custom object as response
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

// 7. Return stream response
#[get("/stream")]
async fn stream() -> HttpResponse {
    let body = once(ok::<_, Error>(Bytes::from_static(b"stream")));

    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(body)
}

// 8. Customize error responses to return 500 server internal error.
#[derive(Fail, Debug)]
#[fail(display = "my error")] // 500 status code with title "my error"
struct MyError {
    name: &'static str,
}
impl error::ResponseError for MyError {}

#[get("/fail")]
async fn fail() -> Result<&'static str, MyError> {
    let err = MyError { name: "test fail" };
    debug!("{}", err);
    Err(err)
}

// 9. Build server error module
#[derive(Fail, Debug)]
enum MyErrors {
    #[fail(display = "internal error")]
    InternalError,
    #[fail(display = "bad request")]
    BadClientData,
    #[fail(display = "timeout")]
    Timeout,
}
impl error::ResponseError for MyErrors {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            MyErrors::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            MyErrors::BadClientData => StatusCode::BAD_REQUEST,
            MyErrors::Timeout => StatusCode::GATEWAY_TIMEOUT,
        }
    }
}

#[get("/bad-data")]
async fn bad_data() -> Result<&'static str, MyErrors> {
    Err(MyErrors::BadClientData)
}

// 10. Build user error module
#[derive(Fail, Debug)]
enum UserErrors {
    #[fail(display = "Validation error on field: {}", field)]
    ValidationError { field: &'static str },
    #[fail(display = "An internal error occurred. Please try again later.")]
    InternalError,
}
impl error::ResponseError for UserErrors {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }
    fn status_code(&self) -> StatusCode {
        match *self {
            UserErrors::ValidationError { .. } => StatusCode::BAD_REQUEST,
            UserErrors::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[get("/user-error")]
async fn user_error() -> Result<&'static str, UserErrors> {
    validate_user_input_error().map_err(|_e| UserErrors::ValidationError { field: "name" })?;
    Ok("success!")
}

fn validate_user_input_error() -> Result<(), MyError> {
    Err(MyError {
        name: "input error",
    })
}

// start point
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        app_name: String::from("hello-actix"),
        counter: Mutex::new(0),
    });

    std::env::set_var("RUST_LOG", "my_errors=debug,actix_web=info");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    // Use App factory to register routes, middlewares and to store state. The shared data has to be thread-safe.
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .service(index)
            .service(custom)
            .service(stream)
            .service(fail)
            .service(bad_data)
            .service(user_error)
            .service(web::scope("/user").configure(user_config)) // Include the configuration.
            .service(web::scope("/app1").app_data(state.clone()).service(app1)) // Clone the state for each thread in the scope.
    })
    .bind("127.0.0.1:8080")?
    .run() // Run and return an instance of the server.
    .await
}
