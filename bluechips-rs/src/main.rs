#[macro_use] extern crate rocket;
use std::collections::HashMap;

use entities::prelude::Currency;
use rocket::fs::FileServer;
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;
use rocket_csrf::{form::CsrfForm, CsrfToken};
use rocket::State;
use askama::Template; // bring trait in scope
use sea_orm::Database;

mod entities;

mod service;
use service::{Query, Mutation, ExpenditureDisplay, TransferDisplay, SettleError, Totals, ExpenditureForm};

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
    settle: Result<Vec<(Option<String>, Option<String>, Currency)>, SettleError>,
    net: Option<Currency>,
    expenditures: Vec<ExpenditureDisplay>,
    transfers: Vec<TransferDisplay>,
    totals: Totals,
}
#[get("/")]
async fn status_index<'a>(db: &State<DatabaseConnection>, flash: Option<FlashMessage<'a>>, user: auth::User) -> StatusIndexTemplate<'a> {
    let db = db as &DatabaseConnection;
    let users: HashMap<_, _> = Query::find_users(db).await.unwrap().into_iter().map(|u| (u.id, u)).collect();
    let debts = Query::get_debts(db).await.unwrap();
    let settle = Query::settle(debts);
    let get_username = |id: i32| Some(id)
        .filter(|id| id != &user.id)
        .map(|id| users.get(&id).map(
            |u| u.name.clone().unwrap_or(u.username.clone())).unwrap_or(format!("{}", id))
        );
    let settle = settle.map(
        |v| v.into_iter().map(
            |(from, to, amount)|
            (get_username(from), get_username(to), amount)
        ).collect::<Vec<_>>());
    let expenditures = Query::find_my_recent_expenditures(db, user.id).await.unwrap();
    let transfers = Query::find_my_recent_transfers(db, user.id).await.unwrap();
    let net = settle.as_ref().ok().map(|settle|
        settle.iter().filter_map(
            |(_, to, amount)| Some(amount.clone()).filter(|_| to.is_none())
        ).sum::<Currency>() - settle.iter().filter_map(
            |(from, _, amount)| Some(amount.clone()).filter(|_| from.is_none())
        ).sum()
    ).filter(|v| *v != 0.into());
    let totals = Query::get_totals(db, user.id).await.unwrap();
    StatusIndexTemplate{title: None, flash, mobile_client: false, settle, net, expenditures, transfers, totals}
}

#[derive(Template)] // this will generate the code...
#[template(path = "spend/index.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct SpendTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    authenticity_token: String,
    users: Vec<entities::user::Model>,
    expenditure: entities::expenditure::ActiveModel,
    splits: HashMap<i32, entities::split::ActiveModel>,
}

#[get("/spend")]
async fn spend_index<'a>(
    db: &State<DatabaseConnection>,
    flash: Option<FlashMessage<'a>>,
    user: auth::User,
    csrf_token: CsrfToken
) -> Result<SpendTemplate<'a>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let users = Query::find_users(db).await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;
    let splits = users.iter().filter(|u| u.resident).map(|u| (u.id, entities::split::ActiveModel{
        user_id: ActiveValue::set(u.id),
        share: ActiveValue::set(100.into()),
        ..Default::default()
    })).collect();
    Ok(SpendTemplate {
        title: Some("Add a New Expenditure"),
        mobile_client: false,
        flash,
        authenticity_token: csrf_token.authenticity_token(),
        users,
        expenditure: entities::expenditure::ActiveModel {
            spender_id: ActiveValue::Set(user.id),
            date: ActiveValue::Set(Some(chrono::Local::now().date_naive())),
            ..Default::default()
        },
        splits,
    })
}
#[get("/spend/<id>/edit")]
async fn spend_edit<'a>(
    id: i32,
    db: &State<DatabaseConnection>,
    flash: Option<FlashMessage<'a>>,
    _user: auth::User,
    csrf_token: CsrfToken
) -> Result<SpendTemplate<'a>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let expenditure =
        entities::expenditure::Entity::find_by_id(id)
            .one(db)
            .await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
            .ok_or(Custom(Status::NotFound, "expenditure not found".to_string()))?;
    let splits =
        expenditure.find_related(entities::split::Entity).all(db).await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;
    let splits = splits.into_iter().map(|s| (s.user_id, s.into_active_model())).collect();
    let users = Query::find_users(db).await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;
    Ok(SpendTemplate {
        title: Some("Edit an Expenditure"),
        mobile_client: false,
        flash,
        authenticity_token: csrf_token.authenticity_token(),
        users,
        expenditure: expenditure.into_active_model(),
        splits,
    })
}
#[get("/spend/<id>/delete")]
fn spend_delete(id: i32) -> Option<()> {
    None
}
#[post("/spend", data="<form>")]
async fn spend_new_post(
    db: &State<DatabaseConnection>,
    user: auth::User,
    form: CsrfForm<ExpenditureForm>,
) -> Result<Flash<Redirect>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let spender = Query::get_user_by_id(db, form.spender_id).await
        .map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
        .ok_or(Custom(Status::BadRequest, "spender not found".to_string()))?;
    Mutation::create_expenditure(db, form.clone()).await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Flash::success(
        Redirect::to(uri!(status_index())),
        format!("Expenditure of {} paid for by {} created.", form.amount, spender.name.unwrap_or(spender.username))
    ))
}
#[post("/spend/<id>", data="<form>")]
async fn spend_edit_post(
    id: i32,
    db: &State<DatabaseConnection>,
    user: auth::User,
    form: CsrfForm<ExpenditureForm>,
) -> Result<Flash<Redirect>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let spender = Query::get_user_by_id(db, form.spender_id).await
        .map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
        .ok_or(Custom(Status::BadRequest, "spender not found".to_string()))?;
    Mutation::update_expenditure(db, id, form.clone()).await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Flash::success(
        Redirect::to(uri!(status_index())),
        format!("Expenditure of {} paid for by {} updated.", form.amount, spender.name.unwrap_or(spender.username))
    ))
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
    Ok(Redirect::to(uri!(status_index())))
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
    HistoryIndexTemplate{title: Some("History"), flash: flash, mobile_client: false, expenditures, transfers}
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
        .mount("/", routes![status_index, spend_index, spend_edit, spend_new_post, spend_edit_post, spend_delete, transfer_index, history_index, user_index, auth_login, auth_login_post])
        .mount("/js", FileServer::from("public/js/"))
        .mount("/css", FileServer::from("public/css/"))
        .mount("/icons", FileServer::from("public/icons/"))
}
