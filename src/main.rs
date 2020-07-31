use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

// 1. Use request handlers to extract parameters from a request(trait:FromRequest) and return a response(trait:Responder).
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

// 2. Use path utility to parse path parameters.
#[get("/{id}/{name}/index.html")]
async fn index_html(info: web::Path<(u32, String)>) -> impl Responder {
    HttpResponse::Ok().body(format!("Hello {}! id:{}", info.1, info.0))
}

// start point
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index).service(index_html))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
