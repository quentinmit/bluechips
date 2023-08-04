#[macro_use] extern crate rocket;
use rocket::fs::FileServer;
use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;
use rocket::State;
use askama::Template; // bring trait in scope
use sea_orm::Database;

mod entities;
use entities::{prelude::*, *};
use sea_orm::*;

#[derive(Template)] // this will generate the code...
#[template(path = "status/index.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct StatusTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    expenditures: Vec<expenditure::Model>,
}

#[get("/")]
async fn status<'a>(db: &State<DatabaseConnection>, flash: Option<FlashMessage<'a>>) -> StatusTemplate<'a> {
    let db = db as &DatabaseConnection;
    let expenditures = Expenditure::find()
        // TODO: spender == user or any (split where user == user and share != 0)
        //.filter(expenditure::Column::SpenderId.eq(1))
        .order_by_desc(expenditure::Column::Date)
        .limit(10)
        .all(db)
        .await.unwrap();
    StatusTemplate{title: None, flash: flash, mobile_client: false, expenditures}
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
