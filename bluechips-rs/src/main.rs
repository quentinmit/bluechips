#[macro_use] extern crate rocket;
use rocket::fs::FileServer;
use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;
use askama::Template; // bring trait in scope
use sea_orm::Database;

mod entities;

use entities::{prelude::*, *};

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
async fn rocket() -> _ {
    let db = Database::connect("sqlite://database.sqlite3").await.unwrap();
    rocket::build()
        .manage(db)
        .mount("/", routes![status])
        .mount("/js", FileServer::from("public/js/"))
        .mount("/css", FileServer::from("public/css/"))
        .mount("/icons", FileServer::from("public/icons/"))
}
