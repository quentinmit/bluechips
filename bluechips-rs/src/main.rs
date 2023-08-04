#[macro_use] extern crate rocket;
use rocket::fs::FileServer;
use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;
use askama::Template; // bring trait in scope
//use askama_rocket;

#[derive(Template)] // this will generate the code...
#[template(path = "base.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct StatusTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
}

#[get("/")]
fn status(flash: Option<FlashMessage<'_>>) -> StatusTemplate<'_> {
    StatusTemplate{title: None, flash: flash, mobile_client: false}
}

#[get("/spend")]
fn spend() -> Option<()> {
    None
}

#[get("/transfer")]
fn transfer() -> Option<()> {
    None
}

#[get("/history")]
fn history() -> Option<()> {
    None
}

#[get("/user")]
fn user() -> Option<()> {
    None
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![status])
        .mount("/js", FileServer::from("public/js/"))
        .mount("/css", FileServer::from("public/css/"))
        .mount("/icons", FileServer::from("public/icons/"))
}
