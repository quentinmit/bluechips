#[macro_use] extern crate rocket;
use rocket::fs::FileServer;
use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;
use rocket::State;
use askama::Template; // bring trait in scope
use sea_orm::Database;
use sea_orm::sea_query::IntoCondition;

mod entities;
use entities::{prelude::*, *};

mod service;
use service::{Query, ExpenditureDisplay};

use sea_orm::{prelude::*, *};

#[derive(Template)] // this will generate the code...
#[template(path = "status/index.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct StatusIndexTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    expenditures: Vec<ExpenditureDisplay>,
}

#[get("/")]
async fn status<'a>(db: &State<DatabaseConnection>, flash: Option<FlashMessage<'a>>) -> StatusIndexTemplate<'a> {
    let db = db as &DatabaseConnection;
    let user_id = 1;
    let expenditures = Query::find_my_recent_expenditures(db, user_id).await.unwrap();
    StatusIndexTemplate{title: None, flash: flash, mobile_client: false, expenditures}
}

#[get("/spend")]
fn spend_index() -> Option<()> {
    None
}
#[get("/spend/<id>/edit")]
fn spend_edit(id: i32) -> Option<()> {
    None
}
#[get("/spend/<id>/delete")]
fn spend_delete(id: i32) -> Option<()> {
    None
}

#[get("/transfer")]
fn transfer_index() -> Option<()> {
    None
}

#[derive(Template)]
#[template(path = "history/index.html")]
struct HistoryIndexTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    expenditures: Vec<ExpenditureDisplay>,
}

#[get("/history")]
async fn history_index<'a>(db: &State<DatabaseConnection>, flash: Option<FlashMessage<'a>>) -> HistoryIndexTemplate<'a> {
    let db = db as &DatabaseConnection;
    let user_id = 1;
    let expenditures = Query::find_all_expenditures(db, user_id).await.unwrap();
    HistoryIndexTemplate{title: None, flash: flash, mobile_client: false, expenditures}
}

#[get("/user")]
fn user_index() -> Option<()> {
    None
}

#[launch]
async fn rocket() -> _ {
    let db = Database::connect("sqlite://database.sqlite3").await.unwrap();
    rocket::build()
        .manage(db)
        .mount("/", routes![status, spend_index, spend_edit, spend_delete, transfer_index, history_index, user_index])
        .mount("/js", FileServer::from("public/js/"))
        .mount("/css", FileServer::from("public/css/"))
        .mount("/icons", FileServer::from("public/icons/"))
}
