use std::path::Path;
use actix_cors::Cors;
use rust_embed::RustEmbed;
use actix_web::{web, App, HttpServer, Responder, HttpResponse, HttpRequest};
use actix_web::middleware::Logger;
use log::info;

#[derive(RustEmbed)]
#[folder = "../target/wasm-bindgen/debug"]
struct FrontendAssets;


async fn index() -> HttpResponse {
    HttpResponse::Ok().content_type("text/html").body(include_str!("../../web/index.html"))
}

async fn catch_all(req: HttpRequest) -> impl Responder {
    info!("catch_all: {}", req.path());
    if req.path().contains("/api/") {
        return HttpResponse::NotFound().finish()
    }
    if req.path().contains("frontend/public") {
        return serve_frontend_asset(req.path().to_string())
    }
    let file_type = match mime_guess::from_path(&req.path()).first_raw() {
        Some(file_type) => file_type,
        None => "text/html"
    };
    if !file_type.contains("text/html") {
        return serve_frontend_asset(req.path().to_string())
    }
    index().await
}

fn serve_frontend_asset(path: String) -> HttpResponse {
    let file = match Path::new(&path).file_name() {
        Some(file) => file.to_str().unwrap(),
        None => return HttpResponse::BadRequest().body("404 Not Found")
    };
    match FrontendAssets::get(file) {
        Some(content) => HttpResponse::Ok()
            .content_type(mime_guess::from_path(&file).first_or_octet_stream().as_ref())
            .append_header(("Cache-Control", "public, max-age=604800"))
            .body(content.data),
        None => HttpResponse::NotFound().body("404 Not Found")
    }
}


#[tokio::main]
async fn main() -> std::io::Result<()> {

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    HttpServer::new(|| {
        let cors = Cors::default().allow_any_origin().allow_any_method().allow_any_header();
        App::new()
            .wrap(cors)
            // .configure()
            .wrap(Logger::new("%a %{User-Agent}i %r %s %D"))
            .default_service(web::route().to(catch_all))
    })
        .bind("0.0.0.0:8001")?
        .run()
        .await
}
