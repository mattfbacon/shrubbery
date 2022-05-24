use actix_web::{get, web::ServiceConfig, HttpResponse, Responder};

#[get("/favicon.ico")]
async fn favicon() -> impl Responder {
	HttpResponse::Ok()
		.insert_header(("Content-Type", "image/x-icon"))
		.body::<&[u8]>(std::include_bytes!("../../static/favicon.ico"))
}

#[get("/res/style/main.css")]
async fn css() -> impl Responder {
	HttpResponse::Ok()
		.insert_header(("Content-Type", "text/css"))
		.body::<&[u8]>(std::include_bytes!("../../static/res/style/main.css"))
}

pub fn configure(app: &mut ServiceConfig) {
	app.service(favicon);
	app.service(css);
}
