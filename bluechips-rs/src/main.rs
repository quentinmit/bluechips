#[macro_use] extern crate rocket;
use entities::prelude::Currency;
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::http::Status;
use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;
use rocket_csrf::{form::CsrfForm, CsrfToken};
use rocket::State;
use askama::Template; // bring trait in scope
use sea_orm::Database;
use sea_orm::sea_query::IntoCondition;

mod entities;

mod service;
use service::{Query, ExpenditureDisplay, TransferDisplay, SettleError, Totals};

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
    settle: Result<Vec<(i32, i32, Currency)>, SettleError>,
    net: Option<Currency>,
    expenditures: Vec<ExpenditureDisplay>,
    transfers: Vec<TransferDisplay>,
    totals: Totals,
}

#[get("/")]
async fn status<'a>(db: &State<DatabaseConnection>, flash: Option<FlashMessage<'a>>, user: auth::User) -> StatusIndexTemplate<'a> {
    let db = db as &DatabaseConnection;
    let debts = Query::get_debts(db, user.id).await.unwrap();
    let settle = Query::settle(debts);
    let expenditures = Query::find_my_recent_expenditures(db, user.id).await.unwrap();
    let transfers = Query::find_my_recent_transfers(db, user.id).await.unwrap();
    let net = settle.as_ref().ok().map(|settle|
        settle.iter().filter_map(
            |(from, to, amount)| Some(amount.clone()).filter(|_| *to == user.id)
        ).sum::<Currency>() - settle.iter().filter_map(
            |(from, to, amount)| Some(amount.clone()).filter(|_| *from == user.id)
        ).sum()
    ).filter(|v| *v != 0.into());
    let totals = Query::get_totals(db, user.id).await.unwrap();
    StatusIndexTemplate{title: None, flash, mobile_client: false, settle, net, expenditures, transfers, totals}
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
    authenticity_token: String,
}

#[get("/login")]
fn auth_login<'a>(flash: Option<FlashMessage<'a>>, csrf_token: CsrfToken) -> AuthLoginTemplate<'a> {
    let authenticity_token = csrf_token.authenticity_token();
    AuthLoginTemplate{title: Some("Login"), flash, mobile_client: false, authenticity_token}
}

#[post("/login", data="<form>")]
async fn auth_login_post<'a>(
    db: &State<DatabaseConnection>,
    form: CsrfForm<auth::Login>,
    auth: auth::Auth<'_>,
) -> Result<Redirect, Flash<Redirect>> {
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
        .attach(rocket_csrf::Fairing::default())
        .manage(db)
        .manage(session_manager)
        .register("/", catchers![unauthorized])
        .mount("/", routes![status, spend_index, spend_edit, spend_delete, transfer_index, history_index, user_index, auth_login, auth_login_post])
        .mount("/js", FileServer::from("public/js/"))
        .mount("/css", FileServer::from("public/css/"))
        .mount("/icons", FileServer::from("public/icons/"))
}
