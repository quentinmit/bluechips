#[macro_use] extern crate rocket;
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::http::Status;
use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;
use rocket::State;
use askama::Template; // bring trait in scope
use sea_orm::Database;
use sea_orm::sea_query::IntoCondition;

mod entities;

mod service;
use service::{Query, ExpenditureDisplay, TransferDisplay};

mod auth;
use auth::SessionManager;

use sea_orm::{prelude::*, *};

#[derive(Template)] // this will generate the code...
#[template(path = "status/index.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct StatusIndexTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    expenditures: Vec<ExpenditureDisplay>,
    transfers: Vec<TransferDisplay>,
}

#[get("/")]
async fn status<'a>(db: &State<DatabaseConnection>, flash: Option<FlashMessage<'a>>, user: auth::User) -> StatusIndexTemplate<'a> {
    let db = db as &DatabaseConnection;
    let expenditures = Query::find_my_recent_expenditures(db, user.id).await.unwrap();
    let transfers = Query::find_my_recent_transfers(db, user.id).await.unwrap();
    StatusIndexTemplate{title: None, flash, mobile_client: false, expenditures, transfers}
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

#[get("/transfer/<id>/edit")]
fn transfer_edit(id: i32) -> Option<()> {
    None
}
#[get("/transfer/<id>/delete")]
fn transfer_delete(id: i32) -> Option<()> {
    None
}

#[derive(Template)] // this will generate the code...
#[template(path = "auth/login.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct AuthLoginTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
}

#[get("/login")]
fn auth_login<'a>(flash: Option<FlashMessage<'a>>) -> AuthLoginTemplate<'a> {
    AuthLoginTemplate{title: Some("Login"), flash, mobile_client: false}
}

#[post("/login", data="<form>")]
async fn auth_login_post<'a>(
    db: &State<DatabaseConnection>,
    form: Form<auth::Login>,
    auth: auth::Auth<'_>
) -> Result<Redirect, Flash<Redirect>> {
    // TODO: XSRF protection
    let db = db as &DatabaseConnection;
    auth.login(&form, db).await
        .map_err(|e| Flash::error(unauthorized(), format!("{:?}", e)))?;
    Ok(Redirect::to(uri!(status())))
}

#[derive(Template)]
#[template(path = "history/index.html")]
struct HistoryIndexTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    expenditures: Vec<ExpenditureDisplay>,
    transfers: Vec<TransferDisplay>,
}

#[get("/history")]
async fn history_index<'a>(db: &State<DatabaseConnection>, flash: Option<FlashMessage<'a>>, user: auth::User) -> HistoryIndexTemplate<'a> {
    let db = db as &DatabaseConnection;
    let expenditures = Query::find_all_expenditures(db, user.id).await.unwrap();
    let transfers = Query::find_all_transfers(db, user.id).await.unwrap();
    HistoryIndexTemplate{title: None, flash: flash, mobile_client: false, expenditures, transfers}
}

#[get("/user")]
fn user_index() -> Option<()> {
    None
}

#[catch(401)]
fn unauthorized() -> Redirect {
    Redirect::to(uri!(auth_login()))
}

#[launch]
async fn rocket() -> _ {
    let db = Database::connect("sqlite://database.sqlite3").await.unwrap();
    let session_manager: Box<dyn SessionManager> = Box::new(chashmap::CHashMap::new());
    rocket::build()
        .manage(db)
        .manage(session_manager)
        .register("/", catchers![unauthorized])
        .mount("/", routes![status, spend_index, spend_edit, spend_delete, transfer_index, history_index, user_index, auth_login, auth_login_post])
        .mount("/js", FileServer::from("public/js/"))
        .mount("/css", FileServer::from("public/css/"))
        .mount("/icons", FileServer::from("public/icons/"))
}
