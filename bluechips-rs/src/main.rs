#[macro_use] extern crate rocket;
use std::collections::HashMap;
use std::path::PathBuf;

use entities::prelude::Currency;
use rocket::Either;
use rocket::fs::FileServer;
use rocket::http::Status;
use cookie::Key;
use rocket::response::status::Custom;
use rocket::response::{Flash, Redirect};
use rocket::request::FlashMessage;
use rocket::fairing::AdHoc;
use rocket::serde::Deserialize;
use rocket_csrf::{form::CsrfForm, CsrfToken};
use rocket::State;
use askama::Template; // bring trait in scope
use sea_orm::Database;

mod entities;

mod service;
use service::{Query, Mutation, ExpenditureDisplay, TransferDisplay, SettleError, Totals, ExpenditureForm, TransferForm};

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
    for (user_id, debt) in &debts {
        info!("User {:?} owes {:?}", user_id, debt);
    }
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

#[derive(Template)] // this will generate the code...
#[template(path = "spend/delete.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct SpendDeleteTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    authenticity_token: String,
    expenditure: ExpenditureDisplay,
}
#[get("/spend/<id>/delete")]
async fn spend_delete<'a>(
    id: i32,
    db: &State<DatabaseConnection>,
    flash: Option<FlashMessage<'a>>,
    user: auth::User,
    csrf_token: CsrfToken,
) -> Result<SpendDeleteTemplate<'a>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let expenditure = Query::get_one_expenditure(db, id, user.id)
            .await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
            .ok_or(Custom(Status::NotFound, "expenditure not found".to_string()))?;
    Ok(SpendDeleteTemplate{
        title: Some("Delete an Expenditure"),
        mobile_client: false,
        flash,
        authenticity_token: csrf_token.authenticity_token(),
        expenditure,
    })
}
#[post("/spend", data="<form>")]
async fn spend_new_post(
    db: &State<DatabaseConnection>,
    user: auth::User,
    form: CsrfForm<ExpenditureForm>,
) -> Result<Flash<Redirect>, Custom<String>> {
    spend_edit_post(None, db, user, form).await
}
#[post("/spend/<id>", data="<form>")]
async fn spend_edit_post(
    id: Option<i32>,
    db: &State<DatabaseConnection>,
    user: auth::User,
    form: CsrfForm<ExpenditureForm>,
) -> Result<Flash<Redirect>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let spender = Query::get_user_by_id(db, form.spender_id).await
        .map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
        .ok_or(Custom(Status::BadRequest, "spender not found".to_string()))?;
    Mutation::save_expenditure(db, id, form.clone()).await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Flash::success(
        Redirect::to(uri!(status_index())),
        format!(
            "Expenditure of {} paid for by {} {}.",
            form.amount,
            spender.name.unwrap_or(spender.username),
            match id {
                Some(_) => "updated",
                None => "created",
            }
        )
    ))
}
#[derive(FromForm, Clone, PartialEq, Eq)]
pub struct DeleteForm<'a> {
    pub delete: Option<&'a str>,
    pub cancel: Option<&'a str>,
}
#[post("/spend/<id>/delete", data="<form>")]
async fn spend_delete_post(
    id: i32,
    db: &State<DatabaseConnection>,
    user: auth::User,
    form: CsrfForm<DeleteForm<'_>>,
) -> Result<Either<Flash<Redirect>, Redirect>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let expenditure =
        Query::get_one_expenditure(db, id, user.id)
            .await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
            .ok_or(Custom(Status::NotFound, "expenditure not found".to_string()))?;
    if form.delete.is_some() {
        // TODO: Make sure foreign key constraints exist on splits
        entities::expenditure::Entity::delete_by_id(expenditure.id)
            .exec(db)
            .await
            .map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;

        Ok(Either::Left(Flash::success(
            Redirect::to(uri!(status_index())),
            format!(
                "Expenditure of {} paid for by {} deleted.",
                expenditure.amount,
                expenditure.spender_name.unwrap_or("me".to_owned()),
            )
        )))
    } else {
        Ok(Either::Right(Redirect::to(uri!(status_index()))))
    }
}

#[derive(Template)] // this will generate the code...
#[template(path = "transfer/index.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct TransferTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    authenticity_token: String,
    users: Vec<entities::user::Model>,
    transfer: entities::transfer::ActiveModel,
}

#[get("/transfer")]
async fn transfer_index<'a>(
    db: &State<DatabaseConnection>,
    flash: Option<FlashMessage<'a>>,
    user: auth::User,
    csrf_token: CsrfToken
) -> Result<TransferTemplate<'a>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let users = Query::find_users(db).await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;
    Ok(TransferTemplate {
        title: Some("Add a New Transfer"),
        mobile_client: false,
        flash,
        authenticity_token: csrf_token.authenticity_token(),
        users,
        transfer: entities::transfer::ActiveModel {
            debtor_id: ActiveValue::Set(user.id),
            date: ActiveValue::Set(Some(chrono::Local::now().date_naive())),
            ..Default::default()
        },
    })
}

#[get("/transfer/<id>/edit")]
async fn transfer_edit<'a>(
    id: i32,
    db: &State<DatabaseConnection>,
    flash: Option<FlashMessage<'a>>,
    _user: auth::User,
    csrf_token: CsrfToken
) -> Result<TransferTemplate<'a>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let transfer =
        entities::transfer::Entity::find_by_id(id)
            .one(db)
            .await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
            .ok_or(Custom(Status::NotFound, "transfer not found".to_string()))?;
    let users = Query::find_users(db).await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;
    Ok(TransferTemplate {
        title: Some("Edit an Expenditure"),
        mobile_client: false,
        flash,
        authenticity_token: csrf_token.authenticity_token(),
        users,
        transfer: transfer.into_active_model(),
    })
}
#[post("/transfer", data="<form>")]
async fn transfer_new_post(
    db: &State<DatabaseConnection>,
    user: auth::User,
    form: CsrfForm<TransferForm>,
) -> Result<Flash<Redirect>, Custom<String>> {
    transfer_edit_post(None, db, user, form).await
}
#[post("/transfer/<id>", data="<form>")]
async fn transfer_edit_post(
    id: Option<i32>,
    db: &State<DatabaseConnection>,
    user: auth::User,
    form: CsrfForm<TransferForm>,
) -> Result<Flash<Redirect>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let debtor = Query::get_user_by_id(db, form.debtor_id).await
        .map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
        .ok_or(Custom(Status::BadRequest, "debtor not found".to_string()))?;
    let creditor = Query::get_user_by_id(db, form.creditor_id).await
        .map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
        .ok_or(Custom(Status::BadRequest, "creditor not found".to_string()))?;
    Mutation::save_transfer(db, id, form.clone()).await.map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Flash::success(
        Redirect::to(uri!(status_index())),
        format!(
            "Transfer of {} from {} to {} {}.",
            form.amount,
            debtor.name.unwrap_or(debtor.username),
            creditor.name.unwrap_or(creditor.username),
            match id {
                Some(_) => "updated",
                None => "created",
            }
        )
    ))
}

#[derive(Template)] // this will generate the code...
#[template(path = "transfer/delete.html")] // using the template in this path, relative
// to the `templates` dir in the crate root
struct TransferDeleteTemplate<'a> { // the name of the struct can be anything
    title: Option<&'a str>,
    mobile_client: bool,
    flash: Option<FlashMessage<'a>>,
    authenticity_token: String,
    transfer: TransferDisplay,
}
#[get("/transfer/<id>/delete")]
async fn transfer_delete<'a>(
    id: i32,
    db: &State<DatabaseConnection>,
    flash: Option<FlashMessage<'a>>,
    user: auth::User,
    csrf_token: CsrfToken,
) -> Result<TransferDeleteTemplate<'a>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let transfer = Query::get_one_transfer(db, id, user.id)
        .await
        .map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
        .ok_or(Custom(Status::NotFound, "transfer not found".to_string()))?;
    Ok(TransferDeleteTemplate{
        title: Some("Delete a Transfer"),
        mobile_client: false,
        flash,
        authenticity_token: csrf_token.authenticity_token(),
        transfer,
    })
}
#[post("/transfer/<id>/delete", data="<form>")]
async fn transfer_delete_post(
    id: i32,
    db: &State<DatabaseConnection>,
    user: auth::User,
    form: CsrfForm<DeleteForm<'_>>,
) -> Result<Either<Flash<Redirect>, Redirect>, Custom<String>> {
    let db = db as &DatabaseConnection;
    let transfer = Query::get_one_transfer(db, id, user.id)
        .await
        .map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?
        .ok_or(Custom(Status::NotFound, "transfer not found".to_string()))?;
    if form.delete.is_some() {
        entities::transfer::Entity::delete_by_id(transfer.id)
            .exec(db)
            .await
            .map_err(|e| Custom(Status::InternalServerError, format!("{:?}", e)))?;

        Ok(Either::Left(Flash::success(
            Redirect::to(uri!(status_index())),
            format!(
                "Transfer of {} from {} to {} deleted.",
                transfer.amount,
                transfer.debtor_name.unwrap_or("me".to_owned()),
                transfer.creditor_name.unwrap_or("me".to_owned()),
            )
        )))
    } else {
        Ok(Either::Right(Redirect::to(uri!(status_index()))))
    }
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

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    db_uri: String,
    public_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_uri: "sqlite://database.sqlite3".to_string(),
            public_path: option_env!("ROCKET_PUBLIC_PATH").unwrap_or("public").into(),
        }
    }
}

#[launch]
async fn rocket() -> _ {
    let figment = rocket::Config::figment()
        .join(("secret_key", Key::generate().master()));
    let config: Config = figment.extract().unwrap();
    let db = Database::connect(config.db_uri).await.unwrap();
    rocket::custom(figment)
        .attach(AdHoc::config::<auth::Config>())
        .attach(rocket_csrf::Fairing::default())
        .manage(db)
        .register("/", catchers![unauthorized])
        .mount("/", routes![
            status_index,
            spend_index,
            spend_edit,
            spend_new_post,
            spend_edit_post,
            spend_delete,
            spend_delete_post,
            transfer_index,
            transfer_edit,
            transfer_new_post,
            transfer_edit_post,
            transfer_delete,
            transfer_delete_post,
            history_index,
            user_index,
            auth_login,
            auth_login_post])
        .mount("/js", FileServer::from(config.public_path.join("js/")))
        .mount("/css", FileServer::from(config.public_path.join("css/")))
        .mount("/icons", FileServer::from(config.public_path.join("icons/")))
}
